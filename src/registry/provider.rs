use crate::registry::cache::CacheManager;
use crate::registry::client::{DocIdResult, ProviderInfo, RegistryClient, RegistryError};
use crate::shared::logging;
use std::sync::Arc;

/// Provider resolver with staged information retrieval and caching
#[derive(Clone)]
pub struct ProviderResolver {
    client: Arc<RegistryClient>,
    cache: Arc<CacheManager>,
}

impl ProviderResolver {
    pub fn new() -> Self {
        Self {
            client: Arc::new(RegistryClient::new()),
            cache: Arc::new(CacheManager::new()),
        }
    }

    /// Stage 1: Resolve provider documentation IDs
    #[allow(dead_code)]
    pub async fn resolve_provider_doc_id(
        &self,
        provider_name: &str,
        provider_namespace: &str,
        service_slug: &str,
        data_type: Option<&str>,
    ) -> Result<Vec<DocIdResult>, RegistryError> {
        let cache_key = format!(
            "docids:{}:{}:{}",
            provider_namespace, provider_name, service_slug
        );

        logging::debug(&format!(
            "Resolving doc IDs for provider {}/{}, service: {}, type: {:?}",
            provider_namespace, provider_name, service_slug, data_type
        ));

        // Check cache first
        if let Some(cached_results) = self.cache.documentation_cache.get(&cache_key).await {
            logging::debug(&format!("Found cached doc IDs for {}", cache_key));
            if let Ok(results) = serde_json::from_str::<Vec<DocIdResult>>(&cached_results) {
                return Ok(results);
            }
        }

        // API call to search for documentation IDs
        let results = self
            .client
            .search_docs(
                provider_name,
                provider_namespace,
                service_slug,
                data_type.unwrap_or("resources"),
            )
            .await?;

        logging::info(&format!(
            "Found {} documentation entries for {}/{} service {}",
            results.len(),
            provider_namespace,
            provider_name,
            service_slug
        ));

        // Cache the results as JSON
        if let Ok(serialized) = serde_json::to_string(&results) {
            self.cache
                .documentation_cache
                .set(cache_key, serialized)
                .await;
        }

        Ok(results)
    }

    /// Stage 2: Get provider documentation content by ID
    pub async fn get_provider_docs(&self, doc_id: &str) -> Result<String, RegistryError> {
        let cache_key = format!("doc:{}", doc_id);

        logging::debug(&format!(
            "Fetching documentation content for ID: {}",
            doc_id
        ));

        // Check cache first
        if let Some(cached_content) = self.cache.documentation_cache.get(&cache_key).await {
            logging::debug(&format!("Found cached content for doc ID: {}", doc_id));
            return Ok(cached_content);
        }

        // API call to get documentation content
        let content = self.client.get_doc_content(doc_id).await?;

        logging::info(&format!(
            "Retrieved documentation content for ID: {} ({} chars)",
            doc_id,
            content.len()
        ));

        // Cache the content
        self.cache
            .documentation_cache
            .set(cache_key, content.clone())
            .await;

        Ok(content)
    }

    /// Search providers with intelligent caching
    pub async fn search_providers(&self, query: &str) -> Result<Vec<ProviderInfo>, RegistryError> {
        let cache_key = format!("search:{}", query);

        logging::debug(&format!("Searching providers with query: {}", query));

        // Check cache for search results (shorter TTL)
        if let Some(cached_results) = self.cache.providers_cache.get(&cache_key).await {
            logging::debug(&format!("Found cached search results for query: {}", query));
            if let Ok(results) = serde_json::from_str::<Vec<ProviderInfo>>(&cached_results) {
                return Ok(results);
            }
        }

        // API call to search providers
        let results = self.client.search_providers(query).await?;

        logging::info(&format!(
            "Search for '{}' returned {} providers",
            query,
            results.len()
        ));

        // Cache search results (shorter TTL for search results)
        if let Ok(serialized) = serde_json::to_string(&results) {
            self.cache.providers_cache.set(cache_key, serialized).await;
        }

        Ok(results)
    }

    /// Get provider information with enhanced error context
    #[allow(dead_code)]
    pub async fn get_provider_info(
        &self,
        provider_name: &str,
        namespace: &str,
    ) -> Result<ProviderInfo, RegistryError> {
        let cache_key = format!("info:{}:{}", namespace, provider_name);

        logging::debug(&format!(
            "Getting provider info for {}/{}",
            namespace, provider_name
        ));

        // Check cache first
        if let Some(cached_info) = self.cache.providers_cache.get(&cache_key).await {
            logging::debug(&format!(
                "Found cached provider info for {}/{}",
                namespace, provider_name
            ));
            if let Ok(info) = serde_json::from_str::<ProviderInfo>(&cached_info) {
                return Ok(info);
            }
        }

        // API call to get provider information
        let info = self
            .client
            .get_provider_info(provider_name, namespace)
            .await?;

        logging::info(&format!(
            "Retrieved provider info for {}/{} - downloads: {}, version: {}",
            namespace, provider_name, info.downloads, info.version
        ));

        // Cache the provider information
        if let Ok(serialized) = serde_json::to_string(&info) {
            self.cache.providers_cache.set(cache_key, serialized).await;
        }

        Ok(info)
    }
}

impl Default for ProviderResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_provider_resolver_creation() {
        let resolver = ProviderResolver::new();
        // Just test that resolver creates successfully
        assert_eq!(Arc::strong_count(&resolver.client), 1);
    }
}
