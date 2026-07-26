//! End-to-end MCP server tests using rmcp's duplex transport.
//!
//! These tests exercise the full MCP protocol lifecycle through the actual
//! transport layer, validating real JSON-RPC message exchange.

use rmcp::{
    ClientHandler, ServerHandler, ServiceExt,
    model::{CallToolRequestParams, ClientInfo, ReadResourceRequestParams, ServerJsonRpcMessage},
    transport::{IntoTransport, Transport},
};
use tfmcp::core::tfmcp::TfMcp;
use tfmcp::mcp::deployment::DeploymentControls;
use tfmcp::mcp::server::TfMcpServer;
use tfmcp::mcp::transport::{CorsMode, HttpSessionMode, HttpTransportConfig};
use tfmcp::shared::security::{SecurityManager, SecurityPolicy};
use tfmcp::tfe::client::TfeClient;

/// Check if Terraform is available on this machine
fn terraform_available() -> bool {
    which::which("terraform").is_ok()
}

/// Create a TfMcpServer backed by a temp Terraform project
async fn setup_server() -> Option<(TfMcpServer, tempfile::TempDir)> {
    if !terraform_available() {
        return None;
    }
    let temp_dir = tempfile::tempdir().ok()?;
    let main_tf = temp_dir.path().join("main.tf");
    tokio::fs::write(
        &main_tf,
        r#"
terraform {
  required_providers {
    local = {
      source  = "hashicorp/local"
      version = "~> 2.1"
    }
  }
}

variable "greeting" {
  type        = string
  default     = "Hello"
  description = "A greeting message"
}

resource "local_file" "test" {
  content  = var.greeting
  filename = "${path.module}/test.txt"
}

output "file_path" {
  value       = local_file.test.filename
  description = "Path to the generated file"
}
"#,
    )
    .await
    .ok()?;

    let dir_str = temp_dir.path().to_string_lossy().to_string();
    let tfmcp = TfMcp::new(None, Some(dir_str)).ok()?;
    Some((
        TfMcpServer::new(tfmcp, tfmcp::mcp::server::ToolFilter::all()),
        temp_dir,
    ))
}

/// Minimal client handler for E2E tests
#[derive(Debug, Clone, Default)]
struct TestClientHandler;

impl ClientHandler for TestClientHandler {
    fn get_info(&self) -> ClientInfo {
        ClientInfo::default()
    }
}

// =============================================================================
// ServerHandler trait tests (direct, no transport)
// =============================================================================

#[tokio::test]
async fn test_server_info_fields() {
    let Some((server, _dir)) = setup_server().await else {
        eprintln!("skipping: terraform not available");
        return;
    };

    let info = server.get_info();

    assert_eq!(info.server_info.name, "tfmcp");
    assert_eq!(info.server_info.version, env!("CARGO_PKG_VERSION"));
    assert!(info.capabilities.tools.is_some());
    assert!(info.capabilities.resources.is_some());
    assert!(info.capabilities.prompts.is_some());
    assert!(info.instructions.is_some());
    assert!(
        info.instructions
            .as_deref()
            .unwrap_or_default()
            .contains("Terraform")
    );
}

// =============================================================================
// E2E tests over duplex transport
// =============================================================================

/// Helper: start server and client over duplex, return the running client peer
async fn start_e2e() -> Option<(
    rmcp::service::RunningService<rmcp::RoleClient, TestClientHandler>,
    tempfile::TempDir,
)> {
    let (server, temp_dir) = setup_server().await?;
    start_e2e_with_server(server, temp_dir).await
}

async fn start_e2e_with_server(
    server: TfMcpServer,
    temp_dir: tempfile::TempDir,
) -> Option<(
    rmcp::service::RunningService<rmcp::RoleClient, TestClientHandler>,
    tempfile::TempDir,
)> {
    let (server_transport, client_transport) = tokio::io::duplex(65536);

    // Spawn server in background
    tokio::spawn(async move {
        let svc = server.serve(server_transport).await.expect("server serve");
        svc.waiting().await.expect("server waiting");
    });

    let client = TestClientHandler
        .serve(client_transport)
        .await
        .expect("client serve");
    Some((client, temp_dir))
}

#[tokio::test]
async fn test_e2e_list_tools() {
    let Some((client, _dir)) = start_e2e().await else {
        eprintln!("skipping: terraform not available");
        return;
    };

    let tools = client.list_tools(None).await.expect("list_tools");

    // Should have all registered tools.
    assert!(
        tools.tools.len() >= 80,
        "Expected at least 80 tools, got {}",
        tools.tools.len()
    );

    // Verify some known tool names exist
    let tool_names: Vec<&str> = tools.tools.iter().map(|t| t.name.as_ref()).collect();
    assert!(tool_names.contains(&"list_terraform_resources"));
    assert!(tool_names.contains(&"get_terraform_plan"));
    assert!(tool_names.contains(&"validate_terraform"));
    assert!(tool_names.contains(&"analyze_terraform"));
    assert!(tool_names.contains(&"inspect_terraform_project"));
    assert!(tool_names.contains(&"detect_terraform_entrypoints"));
    assert!(tool_names.contains(&"get_security_status"));
    assert!(tool_names.contains(&"search_terraform_providers"));
    assert!(tool_names.contains(&"search_providers"));
    assert!(tool_names.contains(&"get_provider_details"));
    assert!(tool_names.contains(&"search_modules"));
    assert!(tool_names.contains(&"analyze_plan"));
    assert!(tool_names.contains(&"review_terraform_plan"));
    assert!(tool_names.contains(&"summarize_plan_for_pr"));
    assert!(tool_names.contains(&"check_provider_lockfile"));
    assert!(tool_names.contains(&"run_terraform_quality_checks"));
    assert!(tool_names.contains(&"terraform_workspace"));
    assert!(tool_names.contains(&"terraform_fmt"));
    assert!(tool_names.contains(&"get_token_permissions"));
    assert!(tool_names.contains(&"list_terraform_orgs"));
    assert!(tool_names.contains(&"list_workspaces"));
    assert!(tool_names.contains(&"get_workspace_details"));
    assert!(tool_names.contains(&"get_plan_json_output"));
    assert!(tool_names.contains(&"search_private_modules"));
    assert!(tool_names.contains(&"get_private_module_details"));
    assert!(tool_names.contains(&"search_private_providers"));
    assert!(tool_names.contains(&"get_private_provider_details"));
    assert!(tool_names.contains(&"create_workspace"));
    assert!(tool_names.contains(&"update_workspace"));
    assert!(tool_names.contains(&"delete_workspace_safely"));
    assert!(tool_names.contains(&"create_run"));
    assert!(tool_names.contains(&"action_run"));
    assert!(tool_names.contains(&"list_workspace_variables"));
    assert!(tool_names.contains(&"create_workspace_variable"));
    assert!(tool_names.contains(&"update_workspace_variable"));
    assert!(tool_names.contains(&"get_workspace_policy_sets"));
    assert!(tool_names.contains(&"attach_policy_set_to_workspace"));
    assert!(tool_names.contains(&"list_variable_sets"));
    assert!(tool_names.contains(&"create_variable_set"));
    assert!(tool_names.contains(&"create_variable_in_variable_set"));
    assert!(tool_names.contains(&"delete_variable_in_variable_set"));
    assert!(tool_names.contains(&"attach_variable_set_to_workspaces"));
    assert!(tool_names.contains(&"detach_variable_set_from_workspaces"));
    assert!(tool_names.contains(&"read_workspace_tags"));
    assert!(tool_names.contains(&"create_workspace_tags"));
    assert!(tool_names.contains(&"list_stacks"));
    assert!(tool_names.contains(&"get_stack_details"));
    assert!(tool_names.contains(&"inspect_state_safety"));
    assert!(tool_names.contains(&"detect_drift_candidates"));
    assert!(tool_names.contains(&"prepare_terraform_change"));

    // Verify each tool has a description
    for tool in &tools.tools {
        assert!(
            tool.description.as_ref().is_some_and(|d| !d.is_empty()),
            "Tool '{}' should have a description",
            tool.name
        );
    }

    let tool = |name: &str| {
        tools
            .tools
            .iter()
            .find(|tool| tool.name == name)
            .unwrap_or_else(|| panic!("tool {name} should exist"))
    };
    for name in [
        "list_terraform_resources",
        "get_terraform_plan",
        "validate_terraform",
        "validate_terraform_detailed",
        "get_terraform_state",
        "analyze_terraform",
        "inspect_terraform_project",
        "detect_terraform_entrypoints",
        "get_security_status",
        "analyze_module_health",
        "get_resource_dependency_graph",
        "suggest_module_refactoring",
        "search_terraform_providers",
        "search_providers",
        "get_provider_info",
        "get_provider_details",
        "get_provider_docs",
        "search_terraform_modules",
        "search_modules",
        "get_module_details",
        "get_latest_module_version",
        "get_latest_provider_version",
        "analyze_plan",
        "review_terraform_plan",
        "summarize_plan_for_pr",
        "analyze_state",
        "terraform_graph",
        "terraform_output",
        "terraform_providers",
        "check_provider_lockfile",
        "run_terraform_quality_checks",
        "inspect_state_safety",
        "detect_drift_candidates",
        "prepare_terraform_change",
        "get_token_permissions",
        "list_terraform_orgs",
        "list_terraform_projects",
        "list_workspaces",
        "get_workspace_details",
        "list_runs",
        "get_run_details",
        "get_plan_details",
        "get_plan_logs",
        "get_plan_json_output",
        "get_apply_details",
        "get_apply_logs",
        "search_private_modules",
        "get_private_module_details",
        "search_private_providers",
        "get_private_provider_details",
        "list_workspace_variables",
        "get_workspace_policy_sets",
        "list_variable_sets",
        "read_workspace_tags",
        "list_stacks",
        "get_stack_details",
        "search_policies",
        "get_policy_details",
        "get_provider_capabilities",
    ] {
        assert_eq!(
            tool(name)
                .annotations
                .as_ref()
                .and_then(|annotations| annotations.read_only_hint),
            Some(true),
            "{name} should be annotated read-only"
        );
    }

    for name in [
        "apply_terraform",
        "destroy_terraform",
        "terraform_import",
        "terraform_taint",
        "terraform_refresh",
        "delete_workspace_safely",
        "action_run",
        "delete_variable_in_variable_set",
    ] {
        assert_eq!(
            tool(name)
                .annotations
                .as_ref()
                .and_then(|annotations| annotations.destructive_hint),
            Some(true),
            "{name} should be annotated destructive"
        );
    }

    for name in [
        "create_workspace",
        "update_workspace",
        "create_run",
        "create_workspace_variable",
        "update_workspace_variable",
        "attach_policy_set_to_workspace",
        "create_variable_set",
        "create_variable_in_variable_set",
        "attach_variable_set_to_workspaces",
        "detach_variable_set_from_workspaces",
        "create_workspace_tags",
    ] {
        let annotations = tool(name)
            .annotations
            .as_ref()
            .unwrap_or_else(|| panic!("{name} should have annotations"));
        assert_ne!(
            annotations.read_only_hint,
            Some(true),
            "{name} must not be annotated read-only"
        );
        assert_eq!(
            annotations.open_world_hint,
            Some(true),
            "{name} should be annotated open-world"
        );
    }
}

#[tokio::test]
async fn test_e2e_gated_tfe_write_tool_fails_closed() {
    let Some((base_server, temp_dir)) = setup_server().await else {
        eprintln!("skipping: terraform not available");
        return;
    };
    drop(base_server);

    let dir_str = temp_dir.path().to_string_lossy().to_string();
    let tfmcp = TfMcp::new(None, Some(dir_str)).expect("tfmcp");
    let disabled_tfe = TfeClient::new_with_operations(
        reqwest::Client::new(),
        "https://app.terraform.io".to_string(),
        None,
        false,
    );
    let audit_log = temp_dir.path().join("audit.log");
    let server = TfMcpServer::new_with_tfe_client_and_audit_manager(
        tfmcp,
        tfmcp::mcp::server::ToolFilter::all(),
        disabled_tfe,
        SecurityManager {
            policy: SecurityPolicy::default(),
            audit_log: Some(audit_log.clone()),
        },
    )
    .with_deployment_controls(DeploymentControls::default());
    let Some((client, _dir)) = start_e2e_with_server(server, temp_dir).await else {
        return;
    };
    let mut args = serde_json::Map::new();
    args.insert("organization".to_string(), serde_json::json!("example-org"));
    args.insert("name".to_string(), serde_json::json!("example-workspace"));

    let result = client
        .call_tool(CallToolRequestParams::new("create_workspace").with_arguments(args))
        .await
        .expect("call_tool create_workspace");

    assert_eq!(result.is_error, Some(true));
    let text = result
        .content
        .first()
        .and_then(|content| content.raw.as_text())
        .map(|text| text.text.as_str())
        .unwrap_or_default();
    assert!(text.contains("ENABLE_TF_OPERATIONS=true"));

    let mut variable_set_args = serde_json::Map::new();
    variable_set_args.insert("organization".to_string(), serde_json::json!("example-org"));
    variable_set_args.insert("name".to_string(), serde_json::json!("example-varset"));

    let variable_set_result = client
        .call_tool(
            CallToolRequestParams::new("create_variable_set").with_arguments(variable_set_args),
        )
        .await
        .expect("call_tool create_variable_set");

    assert_eq!(variable_set_result.is_error, Some(true));
    let variable_set_text = variable_set_result
        .content
        .first()
        .and_then(|content| content.raw.as_text())
        .map(|text| text.text.as_str())
        .unwrap_or_default();
    assert!(variable_set_text.contains("ENABLE_TF_OPERATIONS=true"));

    let audit = tokio::fs::read_to_string(audit_log)
        .await
        .expect("read tfe audit log");
    let audit_entries = audit
        .lines()
        .map(serde_json::from_str::<serde_json::Value>)
        .collect::<Result<Vec<_>, _>>()
        .expect("parse tfe audit entries");
    assert_eq!(audit_entries.len(), 2);
    let audit_entry = &audit_entries[0];
    assert_eq!(audit_entry["operation"], "create_workspace");
    assert_eq!(audit_entry["success"], false);
    assert_eq!(
        audit_entry["command"],
        serde_json::json!(["tfe", "create_workspace"])
    );
    assert_eq!(audit_entries[1]["operation"], "create_variable_set");
    assert_eq!(audit_entries[1]["success"], false);

    let metrics = tfmcp::shared::metrics::snapshot();
    assert!(
        metrics
            .iter()
            .any(|metric| metric.name == "mcp.server.tool.call.errors"),
        "failed TFE tool call should record error metrics"
    );
}

#[tokio::test]
async fn test_e2e_tfe_organization_allowlist_fails_closed() {
    let Some((base_server, temp_dir)) = setup_server().await else {
        eprintln!("skipping: terraform not available");
        return;
    };
    drop(base_server);

    let dir_str = temp_dir.path().to_string_lossy().to_string();
    let tfmcp = TfMcp::new(None, Some(dir_str)).expect("tfmcp");
    let disabled_tfe = TfeClient::new_with_operations(
        reqwest::Client::new(),
        "https://app.terraform.io".to_string(),
        None,
        false,
    );
    let server = TfMcpServer::new_with_tfe_client(
        tfmcp,
        tfmcp::mcp::server::ToolFilter::all(),
        disabled_tfe,
    )
    .with_deployment_controls(DeploymentControls {
        organization_allowlist: vec!["allowed-org".to_string()],
        ..DeploymentControls::default()
    });
    let Some((client, _dir)) = start_e2e_with_server(server, temp_dir).await else {
        return;
    };
    let mut args = serde_json::Map::new();
    args.insert("organization".to_string(), serde_json::json!("blocked-org"));
    args.insert("name".to_string(), serde_json::json!("example-workspace"));

    let result = client
        .call_tool(CallToolRequestParams::new("create_workspace").with_arguments(args))
        .await
        .expect("call_tool create_workspace");

    assert_eq!(result.is_error, Some(true));
    let text = result
        .content
        .first()
        .and_then(|content| content.raw.as_text())
        .map(|text| text.text.as_str())
        .unwrap_or_default();
    assert!(text.contains("MCP_ORGANIZATION_ALLOWLIST"));
    assert!(!text.contains("ENABLE_TF_OPERATIONS=true"));

    for tool in ["get_workspace_details", "delete_workspace_safely"] {
        let mut id_args = serde_json::Map::new();
        id_args.insert("workspace_id".to_string(), serde_json::json!("ws-blocked"));
        let id_result = client
            .call_tool(CallToolRequestParams::new(tool).with_arguments(id_args))
            .await
            .expect("call ID-scoped TFE tool");
        assert_eq!(id_result.is_error, Some(true));
        let id_text = id_result
            .content
            .first()
            .and_then(|content| content.raw.as_text())
            .map(|text| text.text.as_str())
            .unwrap_or_default();
        assert!(id_text.contains("cannot verify account-wide or ID-scoped"));
        assert!(!id_text.contains("TFE_TOKEN"));
    }
}

#[tokio::test]
async fn test_streamable_http_health_and_initialize() {
    let Some((server, _dir)) = setup_server().await else {
        eprintln!("skipping: terraform not available");
        return;
    };

    let config = HttpTransportConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        endpoint: "/mcp".to_string(),
        health_endpoint: "/health".to_string(),
        metrics_endpoint: "/metrics".to_string(),
        cors_mode: CorsMode::Strict,
        allowed_origins: Vec::new(),
        allowed_hosts: Vec::new(),
        heartbeat_interval_secs: Some(15),
        session_mode: HttpSessionMode::Stateless,
        deployment: DeploymentControls::default(),
    };
    let router =
        TfMcpServer::streamable_http_router(server, &config).expect("streamable http router");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind streamable http test listener");
    let addr = listener.local_addr().expect("streamable http test addr");

    let handle = tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("streamable http test server");
    });

    let client = reqwest::Client::new();
    let health: serde_json::Value = client
        .get(format!("http://{addr}/health"))
        .send()
        .await
        .expect("health request")
        .json()
        .await
        .expect("health json");
    assert_eq!(health["status"], "ok");
    assert_eq!(health["transport"], "streamable-http");
    assert_eq!(health["metrics_endpoint"], "/metrics");
    assert_eq!(health["origin_validation_enabled"], true);

    let initialize = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-03-26",
            "capabilities": {},
            "clientInfo": {
                "name": "tfmcp-test",
                "version": "1.0.0"
            }
        }
    });
    let rejected_origin = client
        .post(format!("http://{addr}/mcp"))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("Origin", "https://attacker.example")
        .json(&initialize)
        .send()
        .await
        .expect("hostile-origin initialize request");
    assert_eq!(rejected_origin.status(), reqwest::StatusCode::FORBIDDEN);

    let allowed_origin = client
        .post(format!("http://{addr}/mcp"))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("Origin", "http://localhost")
        .json(&initialize)
        .send()
        .await
        .expect("allowed-origin initialize request");
    assert_eq!(allowed_origin.status(), reqwest::StatusCode::OK);
    assert_eq!(
        allowed_origin
            .headers()
            .get("Access-Control-Allow-Origin")
            .and_then(|value| value.to_str().ok()),
        Some("http://localhost")
    );
    let response: serde_json::Value = allowed_origin.json().await.expect("initialize json");

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["result"]["serverInfo"]["name"], "tfmcp");

    let metrics: serde_json::Value = client
        .get(format!("http://{addr}/metrics"))
        .send()
        .await
        .expect("metrics request")
        .json()
        .await
        .expect("metrics json");
    let metric_names: Vec<_> = metrics
        .as_array()
        .expect("metrics array")
        .iter()
        .filter_map(|metric| metric["name"].as_str())
        .collect();
    assert!(metric_names.contains(&"http.server.request.duration"));

    handle.abort();
}

#[tokio::test]
async fn test_streamable_http_global_rate_limit() {
    let Some((server, _dir)) = setup_server().await else {
        eprintln!("skipping: terraform not available");
        return;
    };

    let config = HttpTransportConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        endpoint: "/mcp".to_string(),
        health_endpoint: "/health".to_string(),
        metrics_endpoint: "/metrics".to_string(),
        cors_mode: CorsMode::Disabled,
        allowed_origins: Vec::new(),
        allowed_hosts: Vec::new(),
        heartbeat_interval_secs: Some(15),
        session_mode: HttpSessionMode::Stateless,
        deployment: DeploymentControls {
            rate_limit_global: Some(1),
            ..DeploymentControls::default()
        },
    };
    let router =
        TfMcpServer::streamable_http_router(server, &config).expect("streamable http router");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind streamable http rate test listener");
    let addr = listener
        .local_addr()
        .expect("streamable http rate test addr");

    let handle = tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("streamable http rate test server");
    });

    let client = reqwest::Client::new();
    let first = client
        .get(format!("http://{addr}/health"))
        .send()
        .await
        .expect("first health request");
    assert_eq!(first.status(), reqwest::StatusCode::OK);

    let second = client
        .get(format!("http://{addr}/health"))
        .send()
        .await
        .expect("second health request");
    assert_eq!(second.status(), reqwest::StatusCode::TOO_MANY_REQUESTS);

    handle.abort();
}

#[tokio::test]
async fn test_e2e_list_resources() {
    let Some((client, _dir)) = start_e2e().await else {
        eprintln!("skipping: terraform not available");
        return;
    };

    let resources = client.list_resources(None).await.expect("list_resources");

    assert_eq!(resources.resources.len(), 5, "Should have 5 MCP resources");

    let uris: Vec<&str> = resources
        .resources
        .iter()
        .map(|r| r.raw.uri.as_str())
        .collect();
    assert!(uris.contains(&"terraform://style-guide"));
    assert!(uris.contains(&"/terraform/style-guide"));
    assert!(uris.contains(&"terraform://module-development"));
    assert!(uris.contains(&"/terraform/module-development"));
    assert!(uris.contains(&"terraform://best-practices"));
}

#[tokio::test]
async fn test_e2e_read_resource() {
    let Some((client, _dir)) = start_e2e().await else {
        eprintln!("skipping: terraform not available");
        return;
    };

    let result = client
        .read_resource(ReadResourceRequestParams::new("terraform://style-guide"))
        .await
        .expect("read_resource");

    assert_eq!(result.contents.len(), 1);
    // The content should contain our style guide text
    let text = &result.contents[0];
    let raw_text = serde_json::to_string(text).unwrap_or_default();
    assert!(
        raw_text.contains("Style Guide"),
        "Should contain style guide content"
    );

    let alias = client
        .read_resource(ReadResourceRequestParams::new("/terraform/style-guide"))
        .await
        .expect("read_resource alias");
    let alias_text = serde_json::to_string(&alias.contents[0]).unwrap_or_default();
    assert!(
        alias_text.contains("Style Guide"),
        "HashiCorp-compatible resource alias should contain style guide content"
    );
}

#[tokio::test]
async fn test_e2e_read_resource_not_found() {
    let Some((client, _dir)) = start_e2e().await else {
        eprintln!("skipping: terraform not available");
        return;
    };

    let result = client
        .read_resource(ReadResourceRequestParams::new("terraform://nonexistent"))
        .await;

    assert!(result.is_err(), "Reading unknown resource should fail");
}

#[tokio::test]
async fn test_e2e_call_tool_list_resources() {
    let Some((client, _dir)) = start_e2e().await else {
        eprintln!("skipping: terraform not available");
        return;
    };

    let result = client
        .call_tool(CallToolRequestParams::new("list_terraform_resources"))
        .await
        .expect("call_tool list_terraform_resources");

    // Should return content (may be an error if terraform not initialized, but
    // the tool call itself should succeed at the MCP protocol level)
    assert!(!result.content.is_empty(), "Should return content");

    // Content should be text
    if let Some(content) = result.content.first() {
        assert!(content.raw.as_text().is_some(), "Content should be text");
    }
}

#[tokio::test]
async fn test_e2e_call_tool_validate() {
    let Some((client, _dir)) = start_e2e().await else {
        eprintln!("skipping: terraform not available");
        return;
    };

    let result = client
        .call_tool(CallToolRequestParams::new("validate_terraform"))
        .await
        .expect("call_tool validate_terraform");

    assert!(
        !result.content.is_empty(),
        "Should return validation content"
    );

    // Content should be parseable text
    if let Some(content) = result.content.first() {
        assert!(content.raw.as_text().is_some(), "Content should be text");
    }
}

#[tokio::test]
async fn test_e2e_call_tool_get_security_status() {
    let Some((client, _dir)) = start_e2e().await else {
        eprintln!("skipping: terraform not available");
        return;
    };

    let result = client
        .call_tool(CallToolRequestParams::new("get_security_status"))
        .await
        .expect("call_tool get_security_status");

    assert!(
        result.is_error.is_none() || result.is_error == Some(false),
        "get_security_status should succeed"
    );

    if let Some(content) = result.content.first() {
        if let Some(text) = content.raw.as_text() {
            let parsed: serde_json::Value =
                serde_json::from_str(&text.text).expect("Should be valid JSON");
            assert!(parsed["policy"].is_object(), "Should have policy field");
            assert!(
                parsed["permissions"].is_object(),
                "Should have permissions field"
            );
            assert!(
                parsed["security_scan"].is_object(),
                "Should have security_scan field"
            );
        }
    }
}

#[tokio::test]
async fn test_e2e_call_tool_with_unknown_name() {
    let Some((client, _dir)) = start_e2e().await else {
        eprintln!("skipping: terraform not available");
        return;
    };

    let result = client
        .call_tool(CallToolRequestParams::new("nonexistent_tool"))
        .await;

    assert!(
        result.is_err(),
        "Calling unknown tool should return an error"
    );
}

#[tokio::test]
async fn test_e2e_call_tool_analyze_module_health() {
    let Some((client, _dir)) = start_e2e().await else {
        eprintln!("skipping: terraform not available");
        return;
    };

    let result = client
        .call_tool(CallToolRequestParams::new("analyze_module_health"))
        .await
        .expect("call_tool analyze_module_health");

    assert!(!result.content.is_empty(), "Should return health analysis");

    if let Some(content) = result.content.first() {
        if let Some(text) = content.raw.as_text() {
            let parsed: serde_json::Value =
                serde_json::from_str(&text.text).expect("Should be valid JSON");
            assert!(
                parsed["health_score"].is_number(),
                "Should have health_score"
            );
        }
    }
}

// =============================================================================
// Raw transport protocol tests
// =============================================================================

fn client_msg(raw: &str) -> rmcp::model::ClientJsonRpcMessage {
    serde_json::from_str(raw).expect("invalid test JSON")
}

#[tokio::test]
async fn test_raw_protocol_initialize() {
    let Some((server, _dir)) = setup_server().await else {
        eprintln!("skipping: terraform not available");
        return;
    };

    let (server_transport, client_transport) = tokio::io::duplex(65536);

    tokio::spawn(async move {
        let svc = server.serve(server_transport).await.expect("serve");
        svc.waiting().await.expect("waiting");
    });

    let mut client = IntoTransport::<rmcp::RoleClient, _, _>::into_transport(client_transport);

    // Send initialize request
    client
        .send(client_msg(
            r#"{
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-03-26",
                    "capabilities": {},
                    "clientInfo": { "name": "test-client", "version": "0.1.0" }
                }
            }"#,
        ))
        .await
        .expect("send initialize");

    // Receive initialize response
    let response = client.receive().await.expect("receive initialize");
    match &response {
        ServerJsonRpcMessage::Response(r) => {
            let json = serde_json::to_value(&r.result).unwrap();
            assert_eq!(
                json["serverInfo"]["name"], "tfmcp",
                "Server name should be tfmcp"
            );
            assert!(
                json["capabilities"]["tools"].is_object(),
                "Should have tools capability"
            );
        }
        other => panic!("Expected Response, got: {other:?}"),
    }

    // Send initialized notification
    client
        .send(client_msg(
            r#"{ "jsonrpc": "2.0", "method": "notifications/initialized" }"#,
        ))
        .await
        .expect("send initialized");

    // Send tools/list request
    client
        .send(client_msg(
            r#"{ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }"#,
        ))
        .await
        .expect("send tools/list");

    let tools_response = client.receive().await.expect("receive tools/list");
    match &tools_response {
        ServerJsonRpcMessage::Response(r) => {
            let json = serde_json::to_value(&r.result).unwrap();
            assert!(json["tools"].is_array(), "Should have tools array");
            let tools = json["tools"].as_array().unwrap();
            assert!(
                tools.len() >= 80,
                "Expected at least 80 tools, got {}",
                tools.len()
            );
        }
        other => panic!("Expected Response, got: {other:?}"),
    }

    // Send resources/list request
    client
        .send(client_msg(
            r#"{ "jsonrpc": "2.0", "id": 3, "method": "resources/list" }"#,
        ))
        .await
        .expect("send resources/list");

    let resources_response = client.receive().await.expect("receive resources/list");
    match &resources_response {
        ServerJsonRpcMessage::Response(r) => {
            let json = serde_json::to_value(&r.result).unwrap();
            assert!(json["resources"].is_array());
            let resources = json["resources"].as_array().unwrap();
            assert_eq!(resources.len(), 5, "Should have 5 resources");
        }
        other => panic!("Expected Response, got: {other:?}"),
    }
}

#[tokio::test]
async fn test_raw_protocol_ping() {
    let Some((server, _dir)) = setup_server().await else {
        eprintln!("skipping: terraform not available");
        return;
    };

    let (server_transport, client_transport) = tokio::io::duplex(65536);
    tokio::spawn(async move {
        let svc = server.serve(server_transport).await.expect("serve");
        svc.waiting().await.expect("waiting");
    });

    let mut client = IntoTransport::<rmcp::RoleClient, _, _>::into_transport(client_transport);

    // Initialize first
    client
        .send(client_msg(
            r#"{
                "jsonrpc": "2.0", "id": 1, "method": "initialize",
                "params": { "protocolVersion": "2025-03-26", "capabilities": {},
                             "clientInfo": { "name": "test", "version": "0.1" } }
            }"#,
        ))
        .await
        .unwrap();
    let _ = client.receive().await.unwrap();

    client
        .send(client_msg(
            r#"{ "jsonrpc": "2.0", "method": "notifications/initialized" }"#,
        ))
        .await
        .unwrap();

    // Send ping
    client
        .send(client_msg(
            r#"{ "jsonrpc": "2.0", "id": 10, "method": "ping" }"#,
        ))
        .await
        .expect("send ping");

    let ping_response = client.receive().await.expect("receive ping");
    match &ping_response {
        ServerJsonRpcMessage::Response(_) => {
            // Ping should return a response (empty result)
        }
        other => panic!("Expected Response to ping, got: {other:?}"),
    }
}
