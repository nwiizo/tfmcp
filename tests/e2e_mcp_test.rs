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
use tfmcp::mcp::server::TfMcpServer;

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

    // Should have all registered tools (21 core + 10 new = 31)
    assert!(
        tools.tools.len() >= 21,
        "Expected at least 21 tools, got {}",
        tools.tools.len()
    );

    // Verify some known tool names exist
    let tool_names: Vec<&str> = tools.tools.iter().map(|t| t.name.as_ref()).collect();
    assert!(tool_names.contains(&"list_terraform_resources"));
    assert!(tool_names.contains(&"get_terraform_plan"));
    assert!(tool_names.contains(&"validate_terraform"));
    assert!(tool_names.contains(&"analyze_terraform"));
    assert!(tool_names.contains(&"get_security_status"));
    assert!(tool_names.contains(&"search_terraform_providers"));
    assert!(tool_names.contains(&"analyze_plan"));
    assert!(tool_names.contains(&"terraform_workspace"));
    assert!(tool_names.contains(&"terraform_fmt"));

    // Verify each tool has a description
    for tool in &tools.tools {
        assert!(
            tool.description.as_ref().is_some_and(|d| !d.is_empty()),
            "Tool '{}' should have a description",
            tool.name
        );
    }
}

#[tokio::test]
async fn test_e2e_list_resources() {
    let Some((client, _dir)) = start_e2e().await else {
        eprintln!("skipping: terraform not available");
        return;
    };

    let resources = client.list_resources(None).await.expect("list_resources");

    assert_eq!(resources.resources.len(), 3, "Should have 3 MCP resources");

    let uris: Vec<&str> = resources
        .resources
        .iter()
        .map(|r| r.raw.uri.as_str())
        .collect();
    assert!(uris.contains(&"terraform://style-guide"));
    assert!(uris.contains(&"terraform://module-development"));
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
                tools.len() >= 21,
                "Expected at least 21 tools, got {}",
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
            assert_eq!(resources.len(), 3, "Should have 3 resources");
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
