use reqwest::{Client, Method, StatusCode};
use serde_json::Value;
use std::future::Future;

mod config;
mod encoding;
mod operations;
mod response;

pub use crate::tfe::types::{
    PageParams, PolicySetWorkspaceAttach, RunCreate, TfeClientStatus, TfeError, VariableAttributes,
    VariableSetCreate, VariableSetVariableCreate, VariableSetVariableDelete, VariableSetWorkspaces,
    WorkspaceCreate, WorkspaceRef, WorkspaceTags, WorkspaceUpdate, WorkspaceVariableCreate,
    WorkspaceVariableUpdate,
};

use config::*;
use encoding::*;
use response::*;

const DEFAULT_TFE_ADDRESS: &str = "https://app.terraform.io";
const DEFAULT_MAX_RESPONSE_BYTES: usize = 64 * 1024;

#[derive(Clone)]
pub struct TfeClient {
    client: Client,
    address: String,
    token: Option<String>,
    operations_enabled: bool,
    max_response_bytes: usize,
}

impl Default for TfeClient {
    fn default() -> Self {
        Self::from_env()
    }
}

impl TfeClient {
    pub fn from_env() -> Self {
        let address = std::env::var("TFE_ADDRESS").unwrap_or_else(|_| DEFAULT_TFE_ADDRESS.into());
        let token = std::env::var("TFE_TOKEN")
            .ok()
            .filter(|value| !value.is_empty());
        let skip_tls_verify = env_bool("TFE_SKIP_TLS_VERIFY");

        let client = Client::builder()
            .danger_accept_invalid_certs(skip_tls_verify)
            .build()
            .unwrap_or_else(|_| Client::new());

        Self::new(client, address, token)
    }

    pub fn new(client: Client, address: String, token: Option<String>) -> Self {
        Self::new_with_operations(client, address, token, operations_enabled())
    }

    pub fn new_with_operations(
        client: Client,
        address: String,
        token: Option<String>,
        operations_enabled: bool,
    ) -> Self {
        Self {
            client,
            address: normalize_address(&address),
            token,
            operations_enabled,
            max_response_bytes: response_byte_limit(),
        }
    }

    pub fn status(&self) -> TfeClientStatus {
        TfeClientStatus {
            address: self.address.clone(),
            token_configured: self.token.is_some(),
        }
    }

    pub async fn get_token_permissions(&self) -> Result<Value, TfeError> {
        self.get_json("/account/details").await
    }

    pub async fn list_organizations(&self, page: PageParams) -> Result<Value, TfeError> {
        self.get_json(&format!("/organizations{}", page.query()))
            .await
    }

    pub fn list_projects<'a>(
        &'a self,
        organization: &'a str,
        page: PageParams,
    ) -> impl Future<Output = Result<Value, TfeError>> + 'a {
        self.get_collection("organizations", organization, "projects", page)
    }

    pub fn list_workspaces<'a>(
        &'a self,
        organization: &'a str,
        page: PageParams,
    ) -> impl Future<Output = Result<Value, TfeError>> + 'a {
        self.get_collection("organizations", organization, "workspaces", page)
    }

    pub async fn get_workspace_by_name(
        &self,
        organization: &str,
        workspace: &str,
    ) -> Result<Value, TfeError> {
        self.get_json(&format!(
            "/organizations/{}/workspaces/{}",
            encode_path_segment(organization),
            encode_path_segment(workspace)
        ))
        .await
    }

    pub async fn get_workspace_by_id(&self, workspace_id: &str) -> Result<Value, TfeError> {
        self.get_json(&format!(
            "/workspaces/{}",
            encode_path_segment(workspace_id)
        ))
        .await
    }

    pub async fn list_runs(&self, workspace_id: &str, page: PageParams) -> Result<Value, TfeError> {
        self.get_collection("workspaces", workspace_id, "runs", page)
            .await
    }

    pub async fn get_run(&self, run_id: &str) -> Result<Value, TfeError> {
        self.get_json(&format!("/runs/{}", encode_path_segment(run_id)))
            .await
    }

    pub async fn get_plan(&self, plan_id: &str) -> Result<Value, TfeError> {
        self.get_json(&format!("/plans/{}", encode_path_segment(plan_id)))
            .await
    }

    pub async fn get_plan_json_output(&self, plan_id: &str) -> Result<Value, TfeError> {
        self.get_json(&format!(
            "/plans/{}/json-output",
            encode_path_segment(plan_id)
        ))
        .await
    }

    pub async fn get_plan_logs(&self, plan_id: &str) -> Result<String, TfeError> {
        let plan = self.get_plan(plan_id).await?;
        let log_url = extract_log_read_url(&plan).ok_or(TfeError::MissingLogReadUrl)?;
        self.get_text_url(&log_url).await
    }

    pub async fn get_apply(&self, apply_id: &str) -> Result<Value, TfeError> {
        self.get_json(&format!("/applies/{}", encode_path_segment(apply_id)))
            .await
    }

    pub async fn get_apply_logs(&self, apply_id: &str) -> Result<String, TfeError> {
        let apply = self.get_apply(apply_id).await?;
        let log_url = extract_log_read_url(&apply).ok_or(TfeError::MissingLogReadUrl)?;
        self.get_text_url(&log_url).await
    }

    async fn get_json(&self, path: &str) -> Result<Value, TfeError> {
        self.json_request(Method::GET, path, None).await
    }

    async fn post_json(&self, path: &str, body: Value) -> Result<Value, TfeError> {
        self.ensure_operations_enabled(path)?;
        self.json_request(Method::POST, path, Some(body)).await
    }

    async fn patch_json(&self, path: &str, body: Value) -> Result<Value, TfeError> {
        self.ensure_operations_enabled(path)?;
        self.json_request(Method::PATCH, path, Some(body)).await
    }

    async fn delete_json(&self, path: &str) -> Result<Value, TfeError> {
        self.ensure_operations_enabled(path)?;
        self.json_request(Method::DELETE, path, None).await
    }

    async fn post_action(&self, path: &str) -> Result<Value, TfeError> {
        self.ensure_operations_enabled(path)?;
        self.json_request(Method::POST, path, None).await
    }

    async fn json_request(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value, TfeError> {
        let token = self.token.as_deref().ok_or(TfeError::MissingToken)?;
        let url = self.api_url(path)?;
        let mut request = self
            .client
            .request(method, url)
            .bearer_auth(token)
            .header("Content-Type", "application/vnd.api+json")
            .header("Accept", "application/vnd.api+json");

        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request.send().await?;
        self.json_response(response).await
    }

    pub async fn list_workspace_variables(&self, workspace_id: &str) -> Result<Value, TfeError> {
        self.get_json(&format!(
            "/workspaces/{}/vars",
            encode_path_segment(workspace_id)
        ))
        .await
    }

    pub async fn get_workspace_policy_sets(&self, workspace_id: &str) -> Result<Value, TfeError> {
        self.get_json(&format!(
            "/workspaces/{}/policy-sets",
            encode_path_segment(workspace_id)
        ))
        .await
    }

    pub async fn list_variable_sets(&self, org: &str, page: PageParams) -> Result<Value, TfeError> {
        self.get_organization_collection(org, "varsets", page).await
    }

    pub async fn read_workspace_tags(&self, workspace_id: &str) -> Result<Value, TfeError> {
        self.get_json(&format!(
            "/workspaces/{}/relationships/tags",
            encode_path_segment(workspace_id)
        ))
        .await
    }

    pub async fn list_stacks(&self, org: &str, page: PageParams) -> Result<Value, TfeError> {
        self.get_organization_collection(org, "stacks", page).await
    }

    pub async fn get_stack(&self, stack_id: &str) -> Result<Value, TfeError> {
        self.get_json(&format!("/stacks/{}", encode_path_segment(stack_id)))
            .await
    }

    fn ensure_operations_enabled(&self, operation: &str) -> Result<(), TfeError> {
        if self.operations_enabled {
            Ok(())
        } else {
            Err(TfeError::OperationDisabled {
                operation: operation.to_string(),
            })
        }
    }

    pub async fn search_registry_items(
        &self,
        organization: &str,
        collection: &str,
        query: Option<&str>,
        registry_name: Option<&str>,
        provider: Option<&str>,
        page: PageParams,
    ) -> Result<Value, TfeError> {
        self.get_json(&registry_collection_path(
            organization,
            collection,
            query,
            registry_name,
            provider,
            page,
        ))
        .await
    }

    pub async fn get_registry_item(
        &self,
        organization: &str,
        collection: &str,
        registry_name: &str,
        namespace: &str,
        name: &str,
        provider: Option<&str>,
    ) -> Result<Value, TfeError> {
        let mut path = format!(
            "/organizations/{}/{}/{}/{}/{}",
            encode_path_segment(organization),
            collection,
            encode_path_segment(registry_name),
            encode_path_segment(namespace),
            encode_path_segment(name)
        );
        if let Some(provider) = provider {
            path.push('/');
            path.push_str(&encode_path_segment(provider));
        }
        self.get_json(&path).await
    }

    async fn get_collection(
        &self,
        scope: &str,
        id: &str,
        collection: &str,
        page: PageParams,
    ) -> Result<Value, TfeError> {
        self.get_json(&format!(
            "/{}/{}/{}{}",
            scope,
            encode_path_segment(id),
            collection,
            page.query()
        ))
        .await
    }

    async fn get_organization_collection(
        &self,
        organization: &str,
        collection: &str,
        page: PageParams,
    ) -> Result<Value, TfeError> {
        self.get_collection("organizations", organization, collection, page)
            .await
    }

    async fn post_relationship_ids(
        &self,
        owner_collection: &str,
        owner_id: &str,
        relationship: &str,
        target_type: &str,
        ids: Vec<String>,
    ) -> Result<Value, TfeError> {
        let path = relationship_collection_path(owner_collection, owner_id, relationship);
        self.post_json(&path, relationship_data_array_body(target_type, ids)?)
            .await
    }

    async fn delete_relationship_id(
        &self,
        owner_collection: &str,
        owner_id: &str,
        relationship: &str,
        target_id: &str,
    ) -> Result<Value, TfeError> {
        let path = format!(
            "{}/{}",
            relationship_collection_path(owner_collection, owner_id, relationship),
            encode_path_segment(target_id)
        );
        self.delete_json(&path).await
    }

    async fn get_text_url(&self, url: &str) -> Result<String, TfeError> {
        let response = self.client.get(url).send().await?;
        let status = response.status();
        let bounded = read_bounded_response(response, self.max_response_bytes).await?;
        let body = if bounded.truncated {
            mark_truncated_text(
                bounded.body,
                bounded.original_bytes,
                self.max_response_bytes,
            )
        } else {
            bounded.body
        };
        if status.is_success() {
            Ok(body)
        } else {
            Err(TfeError::Api { status, body })
        }
    }

    async fn json_response(&self, response: reqwest::Response) -> Result<Value, TfeError> {
        let status = response.status();
        if status == StatusCode::NO_CONTENT {
            return Ok(serde_json::json!({
                "status": status.as_u16(),
                "message": "No content"
            }));
        }

        let bounded = read_bounded_response(response, self.max_response_bytes).await?;
        if !status.is_success() {
            let body = if bounded.truncated {
                mark_truncated_text(
                    bounded.body,
                    bounded.original_bytes,
                    self.max_response_bytes,
                )
            } else {
                bounded.body
            };
            return Err(TfeError::Api { status, body });
        }
        if bounded.truncated {
            return Ok(truncated_json_response_with_original(
                &bounded.body,
                self.max_response_bytes,
                bounded.original_bytes,
            ));
        }
        Ok(serde_json::from_str(&bounded.body)?)
    }

    fn api_url(&self, path: &str) -> Result<String, TfeError> {
        if self.address.is_empty() {
            return Err(TfeError::InvalidAddress(self.address.clone()));
        }

        let path = path.trim_start_matches('/');
        Ok(format!("{}/api/v2/{}", self.address, path))
    }
}

fn workspace_path(
    workspace_id: Option<&str>,
    organization: Option<&str>,
    workspace_name: Option<&str>,
) -> Result<String, TfeError> {
    match (workspace_id, organization, workspace_name) {
        (Some(workspace_id), _, _) => {
            Ok(format!("/workspaces/{}", encode_path_segment(workspace_id)))
        }
        (None, Some(organization), Some(workspace_name)) => Ok(format!(
            "/organizations/{}/workspaces/{}",
            encode_path_segment(organization),
            encode_path_segment(workspace_name)
        )),
        _ => Err(TfeError::InvalidRequest(
            "workspace_id or organization plus workspace_name is required".to_string(),
        )),
    }
}

fn workspace_update_path(input: &WorkspaceUpdate) -> Result<String, TfeError> {
    workspace_path(
        input.workspace_id.as_deref(),
        input.organization.as_deref(),
        input.workspace_name.as_deref(),
    )
}

fn workspace_create_body(input: WorkspaceCreate) -> Result<Value, TfeError> {
    if input.name.trim().is_empty() {
        return Err(TfeError::InvalidRequest(
            "workspace name cannot be empty".to_string(),
        ));
    }

    let mut attributes = serde_json::Map::new();
    attributes.insert("name".to_string(), serde_json::json!(input.name));
    insert_string_attr(&mut attributes, "description", input.description);
    insert_string_attr(
        &mut attributes,
        "terraform-version",
        input.terraform_version,
    );
    insert_string_attr(&mut attributes, "execution-mode", input.execution_mode);
    insert_attr(&mut attributes, "auto-apply", input.auto_apply);

    let mut data = serde_json::Map::from_iter([
        ("type".to_string(), serde_json::json!("workspaces")),
        ("attributes".to_string(), Value::Object(attributes)),
    ]);
    if let Some(project_id) = input.project_id.filter(|value| !value.trim().is_empty()) {
        data.insert(
            "relationships".to_string(),
            serde_json::json!({
                "project": {
                    "data": {
                        "type": "projects",
                        "id": project_id
                    }
                }
            }),
        );
    }

    Ok(serde_json::json!({ "data": data }))
}

fn workspace_update_body(input: WorkspaceUpdate) -> Result<Value, TfeError> {
    let mut attributes = serde_json::Map::new();
    insert_string_attr(&mut attributes, "name", input.new_name);
    insert_string_attr(&mut attributes, "description", input.description);
    insert_string_attr(
        &mut attributes,
        "terraform-version",
        input.terraform_version,
    );
    insert_string_attr(&mut attributes, "execution-mode", input.execution_mode);
    insert_attr(&mut attributes, "auto-apply", input.auto_apply);

    if attributes.is_empty() {
        return Err(TfeError::InvalidRequest(
            "at least one workspace attribute must be provided".to_string(),
        ));
    }

    Ok(serde_json::json!({
        "data": {
            "type": "workspaces",
            "attributes": attributes
        }
    }))
}

fn run_create_body(input: RunCreate) -> Result<Value, TfeError> {
    if input.workspace_id.trim().is_empty() {
        return Err(TfeError::InvalidRequest(
            "workspace_id cannot be empty".to_string(),
        ));
    }

    let mut attributes = serde_json::Map::new();
    insert_string_attr(&mut attributes, "message", input.message);
    insert_string_attr(&mut attributes, "operation", input.operation);
    insert_attr(&mut attributes, "is-destroy", input.is_destroy);
    insert_attr(&mut attributes, "refresh", input.refresh);
    insert_attr(&mut attributes, "plan-only", input.plan_only);
    insert_attr(&mut attributes, "auto-apply", input.auto_apply);

    Ok(serde_json::json!({
        "data": {
            "type": "runs",
            "attributes": attributes,
            "relationships": {
                "workspace": {
                    "data": {
                        "type": "workspaces",
                        "id": input.workspace_id
                    }
                }
            }
        }
    }))
}

fn workspace_variable_path(workspace_id: &str, variable_id: Option<&str>) -> String {
    let mut path = format!("/workspaces/{}/vars", encode_path_segment(workspace_id));
    if let Some(variable_id) = variable_id {
        path.push('/');
        path.push_str(&encode_path_segment(variable_id));
    }
    path
}

fn organization_collection_path(organization: &str, collection: &str) -> String {
    format!(
        "/organizations/{}/{}",
        encode_path_segment(organization),
        collection
    )
}

fn relationship_collection_path(
    owner_collection: &str,
    owner_id: &str,
    relationship: &str,
) -> String {
    format!(
        "/{}/{}/relationships/{}",
        owner_collection,
        encode_path_segment(owner_id),
        relationship
    )
}

fn workspace_variable_body(id: Option<&str>, attributes: serde_json::Map<String, Value>) -> Value {
    let mut data = serde_json::Map::from_iter([
        ("type".to_string(), serde_json::json!("vars")),
        ("attributes".to_string(), Value::Object(attributes)),
    ]);
    if let Some(id) = id {
        data.insert("id".to_string(), serde_json::json!(id));
    }
    serde_json::json!({ "data": data })
}

fn variable_set_body(input: VariableSetCreate) -> Result<Value, TfeError> {
    if input.name.trim().is_empty() {
        return Err(TfeError::InvalidRequest(
            "variable set name cannot be empty".to_string(),
        ));
    }

    let mut attributes = serde_json::Map::new();
    insert_string_attr(&mut attributes, "name", Some(input.name));
    insert_string_attr(&mut attributes, "description", input.description);
    insert_attr(&mut attributes, "global", input.global);

    Ok(serde_json::json!({
        "data": {
            "type": "varsets",
            "attributes": attributes
        }
    }))
}

fn relationship_data_array_body(resource_type: &str, ids: Vec<String>) -> Result<Value, TfeError> {
    relationship_array_body(resource_type, ids, "ids", relationship_id_item)
}

fn relationship_name_array_body(
    resource_type: &str,
    names: Vec<String>,
) -> Result<Value, TfeError> {
    relationship_array_body(resource_type, names, "names", relationship_name_item)
}

fn relationship_id_item(resource_type: &str, id: String) -> Value {
    serde_json::json!({ "type": resource_type, "id": id })
}

fn relationship_name_item(resource_type: &str, name: String) -> Value {
    serde_json::json!({
        "type": resource_type,
        "attributes": { "name": name }
    })
}

fn relationship_array_body<F>(
    resource_type: &str,
    values: Vec<String>,
    field_name: &str,
    render: F,
) -> Result<Value, TfeError>
where
    F: Fn(&str, String) -> Value,
{
    Ok(serde_json::json!({
        "data": non_empty_ids(values, field_name)?
            .into_iter()
            .map(|value| render(resource_type, value))
            .collect::<Vec<_>>()
    }))
}

fn non_empty_ids(values: Vec<String>, field_name: &str) -> Result<Vec<String>, TfeError> {
    let values = values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if values.is_empty() {
        return Err(TfeError::InvalidRequest(format!(
            "{field_name} must contain at least one non-empty value"
        )));
    }
    Ok(values)
}

fn variable_attributes(input: VariableAttributes) -> serde_json::Map<String, Value> {
    let mut attributes = serde_json::Map::new();
    insert_string_attr(&mut attributes, "key", input.key);
    insert_optional_string_attr(&mut attributes, "value", input.value);
    insert_optional_string_attr(&mut attributes, "description", input.description);
    insert_string_attr(&mut attributes, "category", input.category);
    insert_attr(&mut attributes, "hcl", input.hcl);
    insert_attr(&mut attributes, "sensitive", input.sensitive);
    attributes
}

fn insert_optional_string_attr(
    attributes: &mut serde_json::Map<String, Value>,
    name: &str,
    value: Option<String>,
) {
    insert_attr(attributes, name, value);
}

fn insert_string_attr(
    attributes: &mut serde_json::Map<String, Value>,
    name: &str,
    value: Option<String>,
) {
    if let Some(value) = value.filter(|value| !value.trim().is_empty()) {
        insert_attr(attributes, name, Some(value));
    }
}

fn insert_attr<T: serde::Serialize>(
    attributes: &mut serde_json::Map<String, Value>,
    name: &str,
    value: Option<T>,
) {
    if let Some(value) = value {
        attributes.insert(name.to_string(), serde_json::json!(value));
    }
}

fn normalize_run_action(action: &str) -> Result<&'static str, TfeError> {
    match action.trim().to_ascii_lowercase().as_str() {
        "apply" => Ok("apply"),
        "discard" => Ok("discard"),
        "cancel" => Ok("cancel"),
        "force-cancel" | "force_cancel" => Ok("force-cancel"),
        "force-execute" | "force_execute" => Ok("force-execute"),
        other => Err(TfeError::InvalidRequest(format!(
            "unsupported run action '{other}', expected apply, discard, cancel, force-cancel, or force-execute"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_params_are_bounded() {
        assert_eq!(PageParams::new(None, None), PageParams::default());
        assert_eq!(
            PageParams::new(Some(0), Some(200)),
            PageParams {
                number: 1,
                size: 100
            }
        );
    }

    #[test]
    fn status_reports_token_without_exposing_value() {
        let client = TfeClient::new(
            Client::new(),
            "https://app.terraform.io/".to_string(),
            Some("secret-token".to_string()),
        );

        assert_eq!(
            client.status(),
            TfeClientStatus {
                address: "https://app.terraform.io".to_string(),
                token_configured: true,
            }
        );
    }

    #[test]
    fn api_url_uses_api_v2_prefix() {
        let client = TfeClient::new(
            Client::new(),
            "https://tfe.example.com/".to_string(),
            Some("token".to_string()),
        );

        assert_eq!(
            client.api_url("/organizations").unwrap(),
            "https://tfe.example.com/api/v2/organizations"
        );
    }

    #[test]
    fn extracts_log_read_url_from_json_api_document() {
        let value = serde_json::json!({
            "data": {
                "attributes": {
                    "log-read-url": "https://example.com/logs"
                }
            }
        });

        assert_eq!(
            extract_log_read_url(&value),
            Some("https://example.com/logs".to_string())
        );
    }

    #[test]
    fn registry_collection_path_encodes_filters() {
        let path = registry_collection_path(
            "my-org",
            "registry-modules",
            Some("vpc module"),
            Some("private"),
            Some("aws"),
            PageParams::new(Some(2), Some(50)),
        );

        assert_eq!(
            path,
            "/organizations/my-org/registry-modules?page%5Bnumber%5D=2&page%5Bsize%5D=50&q=vpc+module&filter%5Bregistry_name%5D=private&filter%5Bprovider%5D=aws"
        );
    }

    #[test]
    fn truncates_large_json_response_with_preview_metadata() {
        let body = serde_json::json!({
            "data": "x".repeat(128)
        })
        .to_string();

        let truncated = truncated_json_response_with_original(&body, 32, body.len() as u64);

        assert_eq!(truncated["truncated"], true);
        assert_eq!(truncated["max_bytes"], 32);
        assert_eq!(truncated["original_bytes"], body.len());
        assert!(truncated["preview"].as_str().unwrap().len() <= 32);
    }

    #[test]
    fn truncates_large_text_response_with_marker() {
        let body = "abcdef".repeat(32);

        let truncated = mark_truncated_text(body[..24].to_string(), body.len() as u64, 24);

        assert!(truncated.starts_with("abcdefabcdefabcdefabcdef"));
        assert!(truncated.contains("truncated TFE response"));
        assert!(truncated.contains(&body.len().to_string()));
    }

    #[tokio::test]
    async fn write_operation_fails_closed_before_auth() {
        let client = TfeClient::new_with_operations(
            Client::new(),
            "https://app.terraform.io".to_string(),
            None,
            false,
        );

        let error = client
            .create_workspace(WorkspaceCreate {
                organization: "org".to_string(),
                name: "workspace".to_string(),
                description: None,
                terraform_version: None,
                execution_mode: None,
                auto_apply: None,
                project_id: None,
            })
            .await
            .expect_err("disabled operations must fail before auth/network");

        assert!(matches!(error, TfeError::OperationDisabled { .. }));
    }

    #[test]
    fn workspace_create_body_includes_project_relationship() {
        let body = workspace_create_body(WorkspaceCreate {
            organization: "org".to_string(),
            name: "workspace".to_string(),
            description: Some("managed by tfmcp".to_string()),
            terraform_version: Some("1.9.0".to_string()),
            execution_mode: Some("remote".to_string()),
            auto_apply: Some(false),
            project_id: Some("prj-123".to_string()),
        })
        .unwrap();

        assert_eq!(body["data"]["type"], "workspaces");
        assert_eq!(body["data"]["attributes"]["name"], "workspace");
        assert_eq!(
            body["data"]["relationships"]["project"]["data"]["id"],
            "prj-123"
        );
    }

    #[test]
    fn variable_set_body_requires_name_and_uses_json_api_type() {
        let body = variable_set_body(VariableSetCreate {
            organization: "org".to_string(),
            name: "shared".to_string(),
            description: Some("shared variables".to_string()),
            global: Some(false),
        })
        .unwrap();

        assert_eq!(body["data"]["type"], "varsets");
        assert_eq!(body["data"]["attributes"]["name"], "shared");
        assert_eq!(body["data"]["attributes"]["global"], false);

        let err = variable_set_body(VariableSetCreate {
            organization: "org".to_string(),
            name: " ".to_string(),
            description: None,
            global: None,
        })
        .expect_err("empty variable set names should fail");
        assert!(err.to_string().contains("variable set name"));
    }

    #[test]
    fn variable_attributes_preserve_explicit_empty_values() {
        let attributes = variable_attributes(VariableAttributes {
            key: Some("OPTIONAL_VALUE".to_string()),
            value: Some(String::new()),
            description: Some(String::new()),
            category: Some("terraform".to_string()),
            hcl: None,
            sensitive: None,
        });

        assert_eq!(attributes["value"], "");
        assert_eq!(attributes["description"], "");
    }

    #[test]
    fn relationship_bodies_filter_empty_values_and_reject_empty_lists() {
        let body = relationship_data_array_body(
            "workspaces",
            vec!["ws-1".to_string(), " ".to_string(), "ws-2".to_string()],
        )
        .unwrap();
        assert_eq!(body["data"].as_array().unwrap().len(), 2);
        assert_eq!(body["data"][0]["type"], "workspaces");
        assert_eq!(body["data"][0]["id"], "ws-1");

        let tag_body = relationship_name_array_body(
            "tags",
            vec!["prod".to_string(), "".to_string(), "team-a".to_string()],
        )
        .unwrap();
        assert_eq!(tag_body["data"].as_array().unwrap().len(), 2);
        assert_eq!(tag_body["data"][0]["attributes"]["name"], "prod");

        let err = relationship_data_array_body("workspaces", vec![" ".to_string()])
            .expect_err("empty relationship list should fail");
        assert!(err.to_string().contains("must contain at least one"));
    }

    #[test]
    fn unsupported_run_action_is_rejected() {
        let error =
            normalize_run_action("delete").expect_err("unsupported run action should be rejected");

        assert!(matches!(error, TfeError::InvalidRequest(_)));
    }
}
