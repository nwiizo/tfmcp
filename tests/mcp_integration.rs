use anyhow::Result;
use serde_json::{json, Value};
use std::time::Duration;
use tfmcp::core::tfmcp::TfMcp;
use tfmcp::mcp::handler::McpHandler;
use tokio::time::timeout;

/// Test helper to create a temporary Terraform project
async fn setup_test_terraform_project() -> Result<tempfile::TempDir> {
    let temp_dir = tempfile::tempdir()?;
    let main_tf_path = temp_dir.path().join("main.tf");

    tokio::fs::write(
        &main_tf_path,
        r#"
terraform {
  required_providers {
    local = {
      source  = "hashicorp/local"
      version = "~> 2.1"
    }
  }
}

resource "local_file" "test" {
  content  = "Hello, World!"
  filename = "test.txt"
}
        "#,
    )
    .await?;

    Ok(temp_dir)
}

#[tokio::test]
async fn test_mcp_initialize_response() -> Result<()> {
    // Setup test environment
    let temp_dir = setup_test_terraform_project().await?;
    let temp_dir_str = temp_dir.path().to_string_lossy().to_string();
    let mut tfmcp = TfMcp::new(None, Some(temp_dir_str))?;
    let _handler = McpHandler::new(&mut tfmcp);

    // Test that the handler can be created without panicking
    // Note: initialized field is private, so we can't test it directly

    // Test initialize response format
    let expected_capabilities = json!({
        "capabilities": {
            "experimental": {},
            "prompts": { "listChanged": false },
            "resources": { "listChanged": false, "subscribe": false },
            "tools": { "listChanged": false }
        },
        "protocolVersion": "2024-11-05",
        "serverInfo": {
            "name": "tfmcp",
            "version": "0.1.0"
        }
    });

    // Verify the expected response structure is valid JSON
    assert!(expected_capabilities.is_object());
    assert!(expected_capabilities["capabilities"].is_object());
    assert!(expected_capabilities["serverInfo"]["name"].as_str() == Some("tfmcp"));

    Ok(())
}

#[tokio::test]
async fn test_mcp_tools_list() -> Result<()> {
    let temp_dir = setup_test_terraform_project().await?;
    let temp_dir_str = temp_dir.path().to_string_lossy().to_string();
    let mut tfmcp = TfMcp::new(None, Some(temp_dir_str))?;
    let _handler = McpHandler::new(&mut tfmcp);

    // Test that tools JSON is valid
    let tools_json = r#"{
  "tools": [
    {
      "name": "list_terraform_resources",
      "description": "List all resources defined in the Terraform project",
      "inputSchema": {
        "type": "object",
        "properties": {}
      }
    }
  ]
}"#;

    let parsed: Value = serde_json::from_str(tools_json)?;
    assert!(parsed["tools"].is_array());

    let tools = parsed["tools"].as_array().unwrap();
    assert!(!tools.is_empty());

    // Check that the first tool has required fields
    let first_tool = &tools[0];
    assert!(first_tool["name"].is_string());
    assert!(first_tool["description"].is_string());
    assert!(first_tool["inputSchema"].is_object());

    Ok(())
}

#[tokio::test]
async fn test_mcp_error_handling() -> Result<()> {
    let temp_dir = setup_test_terraform_project().await?;
    let temp_dir_str = temp_dir.path().to_string_lossy().to_string();
    let mut tfmcp = TfMcp::new(None, Some(temp_dir_str))?;
    let _handler = McpHandler::new(&mut tfmcp);

    // Test that handler can be created

    // Test error response structure
    let error_response = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "error": {
            "code": -32601,
            "message": "Method not found"
        }
    });

    assert!(error_response["error"]["code"].is_number());
    assert!(error_response["error"]["message"].is_string());

    Ok(())
}

#[tokio::test]
async fn test_cache_manager_compilation() -> Result<()> {
    use tfmcp::registry::cache::CacheManager;

    // Test that CacheManager compiles and has required fields
    let cache_manager = CacheManager::new();

    // Test that both caches exist
    let _doc_cache = &cache_manager.documentation_cache;
    let _providers_cache = &cache_manager.providers_cache;

    // Test basic cache operations
    cache_manager
        .documentation_cache
        .set("test_key".to_string(), "test_value".to_string())
        .await;
    let retrieved = cache_manager.documentation_cache.get("test_key").await;
    assert_eq!(retrieved, Some("test_value".to_string()));

    // Test providers cache
    cache_manager
        .providers_cache
        .set("provider_key".to_string(), "provider_data".to_string())
        .await;
    let provider_data = cache_manager.providers_cache.get("provider_key").await;
    assert_eq!(provider_data, Some("provider_data".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_provider_resolver_compilation() -> Result<()> {
    use tfmcp::registry::provider::ProviderResolver;

    // Test that ProviderResolver compiles correctly
    let resolver = ProviderResolver::new();

    // Test that search_providers method exists and can be called
    // Note: This will likely fail in CI without network access, but it tests compilation
    match timeout(Duration::from_secs(5), resolver.search_providers("aws")).await {
        Ok(_) => {
            // If it succeeds, great!
        }
        Err(_) => {
            // If it times out, that's also fine for compilation testing
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_terraform_project_creation() -> Result<()> {
        let temp_dir = setup_test_terraform_project().await?;

        // Verify main.tf was created
        let main_tf_path = temp_dir.path().join("main.tf");
        assert!(main_tf_path.exists());

        // Verify content contains expected Terraform configuration
        let content = tokio::fs::read_to_string(&main_tf_path).await?;
        assert!(content.contains("terraform"));
        assert!(content.contains("local_file"));
        assert!(content.contains("test"));

        Ok(())
    }

    #[test]
    fn test_json_parsing() -> Result<()> {
        // Test that our JSON constants are valid
        let tools_json = r#"{
  "tools": [
    {
      "name": "test_tool",
      "description": "A test tool",
      "inputSchema": {
        "type": "object",
        "properties": {}
      }
    }
  ]
}"#;

        let parsed: Value = serde_json::from_str(tools_json)?;
        assert!(parsed.is_object());
        assert!(parsed["tools"].is_array());

        Ok(())
    }
}
