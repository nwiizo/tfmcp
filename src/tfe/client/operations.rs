//! Explicitly gated HCP Terraform and Terraform Enterprise write operations.

use super::*;

enum WorkspaceVariableWrite {
    Create(WorkspaceVariableCreate),
    Update(WorkspaceVariableUpdate),
}

impl TfeClient {
    pub async fn create_workspace(&self, input: WorkspaceCreate) -> Result<Value, TfeError> {
        let path = organization_collection_path(&input.organization, "workspaces");
        self.post_json(&path, workspace_create_body(input)?).await
    }

    pub async fn update_workspace(&self, input: WorkspaceUpdate) -> Result<Value, TfeError> {
        self.patch_json(
            &workspace_update_path(&input)?,
            workspace_update_body(input)?,
        )
        .await
    }

    pub async fn safe_delete_workspace(&self, input: WorkspaceRef) -> Result<Value, TfeError> {
        let path = match (
            input.workspace_id.as_deref(),
            input.organization.as_deref(),
            input.workspace_name.as_deref(),
        ) {
            (Some(workspace_id), _, _) => format!(
                "/workspaces/{}/actions/safe-delete",
                encode_path_segment(workspace_id)
            ),
            (None, Some(organization), Some(workspace_name)) => format!(
                "/organizations/{}/workspaces/{}/actions/safe-delete",
                encode_path_segment(organization),
                encode_path_segment(workspace_name)
            ),
            _ => {
                return Err(TfeError::InvalidRequest(
                    "workspace_id or organization plus workspace_name is required".to_string(),
                ));
            }
        };
        self.post_action(&path).await
    }

    pub async fn create_run(&self, input: RunCreate) -> Result<Value, TfeError> {
        self.post_json("/runs", run_create_body(input)?).await
    }

    pub async fn action_run(&self, run_id: &str, action: &str) -> Result<Value, TfeError> {
        let normalized = normalize_run_action(action)?;
        self.post_action(&format!(
            "/runs/{}/actions/{}",
            encode_path_segment(run_id),
            normalized
        ))
        .await
    }

    pub async fn create_workspace_variable(
        &self,
        input: WorkspaceVariableCreate,
    ) -> Result<Value, TfeError> {
        self.write_workspace_variable(WorkspaceVariableWrite::Create(input))
            .await
    }

    pub async fn update_workspace_variable(
        &self,
        input: WorkspaceVariableUpdate,
    ) -> Result<Value, TfeError> {
        self.write_workspace_variable(WorkspaceVariableWrite::Update(input))
            .await
    }

    pub async fn attach_policy_set_to_workspace(
        &self,
        input: PolicySetWorkspaceAttach,
    ) -> Result<Value, TfeError> {
        self.post_relationship_ids(
            "policy-sets",
            &input.policy_set_id,
            "workspaces",
            "workspaces",
            vec![input.workspace_id],
        )
        .await
    }

    pub async fn create_variable_set(&self, input: VariableSetCreate) -> Result<Value, TfeError> {
        let path = organization_collection_path(&input.organization, "varsets");
        self.post_json(&path, variable_set_body(input)?).await
    }

    pub async fn create_variable_in_variable_set(
        &self,
        input: VariableSetVariableCreate,
    ) -> Result<Value, TfeError> {
        let path = relationship_collection_path("varsets", &input.variable_set_id, "vars");
        self.post_json(
            &path,
            workspace_variable_body(None, variable_attributes(input.variable)),
        )
        .await
    }

    pub async fn delete_variable_in_variable_set(
        &self,
        input: VariableSetVariableDelete,
    ) -> Result<Value, TfeError> {
        self.delete_relationship_id(
            "varsets",
            &input.variable_set_id,
            "vars",
            &input.variable_id,
        )
        .await
    }

    pub async fn attach_variable_set_to_workspaces(
        &self,
        input: VariableSetWorkspaces,
    ) -> Result<Value, TfeError> {
        self.post_relationship_ids(
            "varsets",
            &input.variable_set_id,
            "workspaces",
            "workspaces",
            input.workspace_ids,
        )
        .await
    }

    pub async fn detach_variable_set_from_workspaces(
        &self,
        input: VariableSetWorkspaces,
    ) -> Result<Value, TfeError> {
        let ids = non_empty_ids(input.workspace_ids, "workspace_ids")?;
        let mut results = Vec::new();
        for workspace_id in ids {
            let deleted = self
                .delete_relationship_id(
                    "varsets",
                    &input.variable_set_id,
                    "workspaces",
                    &workspace_id,
                )
                .await?;
            results.push(deleted);
        }
        Ok(serde_json::json!({ "data": results }))
    }

    pub async fn create_workspace_tags(&self, input: WorkspaceTags) -> Result<Value, TfeError> {
        let path = relationship_collection_path("workspaces", &input.workspace_id, "tags");
        self.post_json(&path, relationship_name_array_body("tags", input.tags)?)
            .await
    }

    async fn write_workspace_variable(
        &self,
        write: WorkspaceVariableWrite,
    ) -> Result<Value, TfeError> {
        let (method, path, variable_id, variable) = match write {
            WorkspaceVariableWrite::Create(input) => (
                Method::POST,
                workspace_variable_path(&input.workspace_id, None),
                None,
                input.variable,
            ),
            WorkspaceVariableWrite::Update(input) => {
                let path = workspace_variable_path(&input.workspace_id, Some(&input.variable_id));
                (Method::PATCH, path, Some(input.variable_id), input.variable)
            }
        };
        let body = workspace_variable_body(variable_id.as_deref(), variable_attributes(variable));
        match method {
            Method::POST => self.post_json(&path, body).await,
            Method::PATCH => self.patch_json(&path, body).await,
            other => Err(TfeError::InvalidRequest(format!(
                "unsupported variable write method: {other}"
            ))),
        }
    }
}
