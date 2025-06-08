// Compilation tests to prevent regressions in core types and structures

#[cfg(test)]
mod compilation_tests {
    use std::time::Duration;

    #[test]
    fn test_cache_manager_fields() {
        use tfmcp::registry::cache::CacheManager;

        // This test ensures CacheManager has all required fields
        let cache_manager = CacheManager::new();

        // Test that accessing these fields compiles
        let _doc_cache = &cache_manager.documentation_cache;
        let _providers_cache = &cache_manager.providers_cache;

        // Test that the fields have the correct types
        assert_eq!(
            std::any::type_name_of_val(&cache_manager.documentation_cache),
            std::any::type_name::<tfmcp::registry::cache::DocumentationCache>()
        );
        assert_eq!(
            std::any::type_name_of_val(&cache_manager.providers_cache),
            std::any::type_name::<tfmcp::registry::cache::ProvidersCache>()
        );
    }

    #[test]
    fn test_simple_cache_interface() {
        use tfmcp::registry::cache::SimpleCache;

        // Test that SimpleCache has the required methods
        let cache = SimpleCache::<String>::new(Duration::from_secs(300));

        // These should compile - we're testing the interface exists
        let _ = std::any::type_name_of_val(&cache);

        // Test that the cache type is correct
    }

    #[test]
    fn test_provider_resolver_interface() {
        use tfmcp::registry::provider::ProviderResolver;

        // Test that ProviderResolver compiles and has expected methods
        let resolver = ProviderResolver::new();

        // Test Default trait implementation
        let _default_resolver = ProviderResolver::default();

        // Test Clone trait (if implemented)
        let _cloned = resolver.clone();
    }

    #[test]
    fn test_mcp_handler_interface() {
        // Test that we can create the types without runtime dependencies
        use tfmcp::core::tfmcp::TfMcp;

        // Test that TfMcp::new compiles with proper arguments
        let temp_dir = std::env::temp_dir().to_string_lossy().to_string();
        let result = TfMcp::new(None, Some(temp_dir));

        // We don't care if it succeeds or fails, just that it compiles
        match result {
            Ok(_) => println!("TfMcp creation succeeded in test"),
            Err(_) => println!("TfMcp creation failed in test (expected in some environments)"),
        }
    }

    #[test]
    fn test_registry_client_interface() {
        use tfmcp::registry::client::RegistryClient;
        use tfmcp::registry::fallback::RegistryClientWithFallback;

        // Test that these types can be instantiated
        let _client = RegistryClient::new();
        let _fallback_client = RegistryClientWithFallback::new();
    }

    #[test]
    fn test_error_types_compilation() {
        use tfmcp::registry::client::RegistryError;

        // Test that error types implement required traits
        let error = RegistryError::HttpError("test".to_string());

        // Test Debug implementation
        let _debug_str = format!("{:?}", error);

        // Test Clone implementation
        let _cloned = error.clone();
    }

    #[test]
    fn test_stdio_transport_types() {
        use serde_json::json;
        use tfmcp::mcp::stdio::Message;

        // Test that Message enum variants compile
        let _request = Message::Request {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "test".to_string(),
            params: Some(json!({})),
        };

        let _response = Message::Response {
            jsonrpc: "2.0".to_string(),
            id: 1,
            result: Some(json!({"status": "ok"})),
            error: None,
        };

        let _notification = Message::Notification {
            jsonrpc: "2.0".to_string(),
            method: "test".to_string(),
            params: Some(json!({})),
        };

        // Note: StdioTransport::new() requires tokio runtime, so we skip it in sync tests
    }

    #[test]
    fn test_terraform_service_types() {
        use std::path::PathBuf;
        use tfmcp::terraform::service::TerraformService;

        // Test that TerraformService can be instantiated
        let temp_dir = std::env::temp_dir();
        let terraform_path = PathBuf::from("terraform");

        let service = TerraformService::new(terraform_path, temp_dir);

        // Test that the service was created
        let _ = service;
        println!("TerraformService creation completed");
    }

    #[test]
    fn test_formatters_compilation() {
        use tfmcp::formatters::output::OutputFormatter;

        // Test that OutputFormatter exists and can be used
        let formatter = OutputFormatter;

        // Test that formatter can be created
        let _ = formatter;
        println!("OutputFormatter compilation test passed");
    }

    #[test]
    fn test_prompts_compilation() {
        use tfmcp::prompts::builder::ToolDescription;

        // Test that ToolDescription can be created and used
        let tool_desc = ToolDescription::new("test_tool");

        // Test that builder methods exist
        let _with_usage = tool_desc.with_usage_guide("Usage: test_tool <args>");

        // Test method chaining compiles
        let _full_desc = ToolDescription::new("test")
            .with_usage_guide("Usage guide")
            .with_constraint("constraint1")
            .with_security_note("security note");
    }
}

/// Test module for async compilation tests
#[cfg(test)]
mod async_compilation_tests {
    use tokio::test;

    #[test]
    async fn test_async_cache_operations() {
        use std::time::Duration;
        use tfmcp::registry::cache::{CacheManager, SimpleCache};

        let cache = SimpleCache::<String>::new(Duration::from_secs(10));

        // Test that async methods compile
        cache.set("key".to_string(), "value".to_string()).await;
        let _result = cache.get("key").await;
        cache.clear().await;
        let _stats = cache.stats().await;
        cache.cleanup_expired().await;

        // Test CacheManager async methods
        let manager = CacheManager::new();
        let _global_stats = manager.global_stats().await;
        manager.cleanup_all().await;
        manager.clear_all().await;
    }

    #[test]
    async fn test_async_provider_operations() {
        use tfmcp::registry::provider::ProviderResolver;

        let resolver = ProviderResolver::new();

        // Test that these methods compile (they may fail at runtime without network)
        let _search_result = resolver.search_providers("test").await;
        let _provider_info = resolver.get_provider_info("test", "hashicorp").await;
        let _doc_ids = resolver
            .resolve_provider_doc_id("test", "hashicorp", "resource", None)
            .await;
        let _docs = resolver.get_provider_docs("test_id").await;
    }

    #[test]
    async fn test_async_registry_client() {
        use tfmcp::registry::client::RegistryClient;
        use tfmcp::registry::fallback::RegistryClientWithFallback;

        let client = RegistryClient::new();
        let fallback_client = RegistryClientWithFallback::new();

        // Test that async methods compile (these may fail at runtime without network)
        let _providers = client.search_providers("test").await;
        let _info = client.get_provider_info("test", "hashicorp").await;
        let _docs = client
            .search_docs("test", "hashicorp", "resource", "resources")
            .await;

        // Test fallback client methods
        let _fallback_info = fallback_client.get_provider_info("test", None).await;
        let _fallback_docs = fallback_client
            .search_docs_with_fallback("test", None, "resource", "resources")
            .await;
    }
}
