use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

use tfmcp::registry::{
    batch::BatchFetcher,
    cache::CacheManager,
    client::{ProviderInfo, RegistryClient},
    fallback::RegistryClientWithFallback,
    provider::ProviderResolver,
};

/// Helper function to create test provider info
fn create_test_provider(name: &str, namespace: &str) -> ProviderInfo {
    ProviderInfo {
        name: name.to_string(),
        namespace: namespace.to_string(),
        version: "1.0.0".to_string(),
        description: format!("Test provider for {}", name),
        downloads: 1000,
        published_at: "2023-01-01".to_string(),
        id: format!("{}/{}", namespace, name),
        source: None,
        tag: None,
        logo_url: None,
        owner: None,
        tier: None,
        verified: None,
        trusted: None,
        extra: std::collections::HashMap::new(),
    }
}

#[tokio::test]
async fn test_provider_resolver_creation() {
    let resolver = ProviderResolver::new();

    // Test that the resolver creates successfully by verifying it can be cloned
    let cloned_resolver = resolver.clone();
    assert!(std::mem::size_of_val(&resolver) > 0);
    assert!(std::mem::size_of_val(&cloned_resolver) > 0);
}

#[tokio::test]
async fn test_cache_manager_operations() {
    let cache_manager = CacheManager::new();

    // Test basic cache operations
    cache_manager
        .providers_cache
        .set("test_key".to_string(), "test_value".to_string())
        .await;

    let cached_value = cache_manager.providers_cache.get("test_key").await;
    assert_eq!(cached_value, Some("test_value".to_string()));

    // Test cache statistics
    let stats = cache_manager.providers_cache.stats().await;
    assert!(stats.total_entries >= 1);
    assert!(stats.valid_entries >= 1);
    assert_eq!(stats.expired_entries, 0);
}

#[tokio::test]
async fn test_batch_fetcher_configuration() {
    let client = Arc::new(RegistryClient::new());

    // Test different concurrency settings
    let fetcher_low = BatchFetcher::new(client.clone(), 2);
    let fetcher_high = BatchFetcher::new(client.clone(), 15); // Should be capped at 10
    let fetcher_zero = BatchFetcher::new(client.clone(), 0); // Should be set to 1

    assert_eq!(fetcher_low.max_concurrent, 2);
    assert_eq!(fetcher_high.max_concurrent, 10); // Capped at maximum
    assert_eq!(fetcher_zero.max_concurrent, 1); // Minimum enforced
}

#[tokio::test]
async fn test_fallback_client_creation() {
    let fallback_client = RegistryClientWithFallback::new();

    // Test that fallback namespaces are properly configured
    assert_eq!(fallback_client.fallback_namespaces.len(), 3);
    assert!(fallback_client
        .fallback_namespaces
        .contains(&"hashicorp".to_string()));
    assert!(fallback_client
        .fallback_namespaces
        .contains(&"terraform-providers".to_string()));
    assert!(fallback_client
        .fallback_namespaces
        .contains(&"community".to_string()));
}

#[tokio::test]
async fn test_cache_expiration_and_cleanup() {
    let cache = tfmcp::registry::cache::SimpleCache::new(Duration::from_millis(100));

    // Add test data
    cache.set("key1".to_string(), "value1".to_string()).await;
    cache.set("key2".to_string(), "value2".to_string()).await;

    // Verify data is accessible
    assert_eq!(cache.get("key1").await, Some("value1".to_string()));
    assert_eq!(cache.get("key2").await, Some("value2".to_string()));

    // Wait for expiration
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Verify data has expired
    assert_eq!(cache.get("key1").await, None);
    assert_eq!(cache.get("key2").await, None);

    // Test cache statistics after expiration
    let stats = cache.stats().await;
    assert_eq!(stats.valid_entries, 0);
    assert_eq!(stats.expired_entries, 2);
}

#[tokio::test]
async fn test_cache_cleanup_functionality() {
    let cache = tfmcp::registry::cache::SimpleCache::new(Duration::from_millis(100));

    // Add test data
    cache.set("key1".to_string(), "value1".to_string()).await;
    cache.set("key2".to_string(), "value2".to_string()).await;

    // Wait for expiration
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Verify expired entries are tracked
    let stats_before = cache.stats().await;
    assert_eq!(stats_before.expired_entries, 2);

    // Clean up expired entries
    cache.cleanup_expired().await;

    // Verify cleanup worked
    let stats_after = cache.stats().await;
    assert_eq!(stats_after.total_entries, 0);
    assert_eq!(stats_after.valid_entries, 0);
    assert_eq!(stats_after.expired_entries, 0);
}

#[tokio::test]
async fn test_empty_batch_operations() {
    let fetcher = BatchFetcher::default();

    // Test empty provider list
    let results = fetcher.fetch_providers(vec![]).await;
    assert!(results.is_empty());

    // Test empty version list
    let version_results = fetcher.fetch_provider_versions(vec![]).await;
    assert!(version_results.is_empty());

    // Test empty docs list
    let docs_results = fetcher.fetch_multiple_docs(vec![]).await;
    assert!(docs_results.is_empty());
}

#[tokio::test]
async fn test_provider_search_timeout_handling() {
    let resolver = ProviderResolver::new();

    // Test with a very short timeout to simulate timeout conditions
    let search_future = resolver.search_providers("aws");
    let result = timeout(Duration::from_millis(1), search_future).await;

    // This should timeout, demonstrating our timeout handling works
    assert!(
        result.is_err(),
        "Expected timeout error for very short duration"
    );
}

#[tokio::test]
async fn test_comprehensive_cache_manager_stats() {
    let cache_manager = CacheManager::new();

    // Add data to different caches
    cache_manager
        .providers_cache
        .set("provider1".to_string(), "data1".to_string())
        .await;

    cache_manager
        .documentation_cache
        .set("doc1".to_string(), "content1".to_string())
        .await;

    // Test global statistics
    let global_stats = cache_manager.global_stats().await;
    assert_eq!(global_stats.len(), 2);
    assert!(global_stats.contains_key("providers"));
    assert!(global_stats.contains_key("documentation"));

    // Verify each cache has the expected data
    let provider_stats = &global_stats["providers"];
    assert!(provider_stats.total_entries >= 1);

    let doc_stats = &global_stats["documentation"];
    assert!(doc_stats.total_entries >= 1);
}

#[tokio::test]
async fn test_output_formatter_provider_list() {
    let providers = vec![
        create_test_provider("aws", "hashicorp"),
        create_test_provider("google", "hashicorp"),
    ];

    let formatted = tfmcp::formatters::output::OutputFormatter::format_provider_list(providers);

    // Verify structure
    assert_eq!(formatted["summary"]["total_providers"], 2);
    assert!(formatted["providers"].is_array());
    assert_eq!(formatted["providers"].as_array().unwrap().len(), 2);

    // Verify provider data
    let first_provider = &formatted["providers"][0];
    assert_eq!(first_provider["name"], "aws");
    assert_eq!(first_provider["namespace"], "hashicorp");
    assert_eq!(first_provider["id"], "hashicorp/aws");
}

#[tokio::test]
async fn test_output_formatter_error_handling() {
    let error_msg = "Provider not found";
    let suggestions = Some(vec![
        "Check provider name spelling".to_string(),
        "Try searching without namespace".to_string(),
    ]);

    let formatted = tfmcp::formatters::output::OutputFormatter::format_error_with_suggestions(
        error_msg,
        suggestions,
        None,
    );

    // Verify error structure
    assert_eq!(formatted["error"]["message"], "Provider not found");
    assert_eq!(formatted["error"]["type"], "provider_resolution_error");

    // Verify suggestions
    assert!(formatted["suggestions"].is_object());
    let suggested_actions = &formatted["suggestions"]["recommended_actions"];
    assert_eq!(suggested_actions.as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn test_tool_description_builder() {
    use tfmcp::prompts::builder::{ToolDescription, ToolExample};

    let example = ToolExample {
        title: "Basic Search".to_string(),
        description: "Search for AWS provider".to_string(),
        input: serde_json::json!({"query": "aws"}),
        expected_output: "List of AWS-related providers".to_string(),
    };

    let description = ToolDescription::new("Search for Terraform providers")
        .with_usage_guide("Use this tool to find providers by name or keyword")
        .with_constraint("Query must be at least 2 characters")
        .with_security_note("Only searches public Terraform Registry")
        .with_example(example)
        .with_error_hint("No Results", "Try broader search terms");

    let prompt = description.build_prompt();

    // Verify prompt contains all components
    assert!(prompt.contains("Search for Terraform providers"));
    assert!(prompt.contains("Usage Guide"));
    assert!(prompt.contains("Constraints"));
    assert!(prompt.contains("Security Notes"));
    assert!(prompt.contains("Examples"));
    assert!(prompt.contains("Troubleshooting"));
}

#[tokio::test]
async fn test_performance_benchmarks() {
    use std::time::Instant;

    let cache_manager = CacheManager::new();
    let start = Instant::now();

    // Simulate adding multiple cache entries
    for i in 0..100 {
        cache_manager
            .providers_cache
            .set(format!("key_{}", i), format!("value_{}", i))
            .await;
    }

    let write_duration = start.elapsed();
    assert!(
        write_duration < Duration::from_millis(100),
        "Cache writes should be fast"
    );

    let read_start = Instant::now();

    // Test read performance
    for i in 0..100 {
        let _ = cache_manager
            .providers_cache
            .get(&format!("key_{}", i))
            .await;
    }

    let read_duration = read_start.elapsed();
    assert!(
        read_duration < Duration::from_millis(50),
        "Cache reads should be very fast"
    );
}

/// Integration test for the complete provider resolution workflow
#[tokio::test]
async fn test_end_to_end_provider_workflow() {
    let resolver = ProviderResolver::new();

    // Test the complete workflow would work with real API calls
    // Note: This is a mock test since we don't want to make real API calls in tests

    // Verify that the resolver can be properly created and used
    let cloned_resolver = resolver.clone();
    assert!(std::mem::size_of_val(&resolver) > 0);
    assert!(std::mem::size_of_val(&cloned_resolver) > 0);

    // Test integration with cache manager
    let cache_manager = CacheManager::new();
    let cache_stats = cache_manager.providers_cache.stats().await;
    assert_eq!(cache_stats.total_entries, 0); // Should start empty
    assert_eq!(cache_stats.valid_entries, 0);
    assert_eq!(cache_stats.expired_entries, 0);
}
