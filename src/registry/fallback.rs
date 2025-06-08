use crate::registry::client::{ProviderInfo, RegistryClient, RegistryError};
use crate::shared::logging;
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FallbackError {
    #[error("Provider '{provider}' not found in any namespace. Searched: {namespaces:?}")]
    ProviderNotFoundAnywhere {
        provider: String,
        namespaces: Vec<String>,
    },

    #[error("Registry error: {0}")]
    RegistryError(#[from] RegistryError),
}

/// Registry client with intelligent fallback capabilities
pub struct RegistryClientWithFallback {
    primary: Arc<RegistryClient>,
    pub fallback_namespaces: Vec<String>,
}

impl RegistryClientWithFallback {
    pub fn new() -> Self {
        Self {
            primary: Arc::new(RegistryClient::new()),
            fallback_namespaces: vec![
                "hashicorp".to_string(),
                "terraform-providers".to_string(),
                "community".to_string(),
            ],
        }
    }

    /// Get provider version with intelligent fallback
    /// Tries the specified namespace first, then falls back to common namespaces
    pub async fn get_provider_version(
        &self,
        provider: &str,
        namespace: Option<&str>,
    ) -> Result<(String, String), FallbackError> {
        let mut searched_namespaces = Vec::new();

        // First, try the specified namespace if provided
        if let Some(ns) = namespace {
            searched_namespaces.push(ns.to_string());
            match self.primary.get_latest_version(provider, ns).await {
                Ok(version) => {
                    logging::info(&format!(
                        "Found provider {} in specified namespace {} with version {}",
                        provider, ns, version
                    ));
                    return Ok((version, ns.to_string()));
                }
                Err(RegistryError::ProviderNotFound { .. }) => {
                    logging::debug(&format!(
                        "Provider {} not found in specified namespace {}, trying fallbacks",
                        provider, ns
                    ));
                }
                Err(e) => return Err(FallbackError::RegistryError(e)),
            }
        }

        // Try fallback namespaces
        for fallback_ns in &self.fallback_namespaces {
            // Skip if we already tried this namespace
            if namespace.is_some_and(|ns| ns == fallback_ns) {
                continue;
            }

            searched_namespaces.push(fallback_ns.clone());
            match self.primary.get_latest_version(provider, fallback_ns).await {
                Ok(version) => {
                    logging::info(&format!(
                        "Found provider {} in fallback namespace {} with version {}",
                        provider, fallback_ns, version
                    ));
                    return Ok((version, fallback_ns.clone()));
                }
                Err(RegistryError::ProviderNotFound { .. }) => {
                    logging::debug(&format!(
                        "Provider {} not found in fallback namespace {}",
                        provider, fallback_ns
                    ));
                    continue;
                }
                Err(e) => return Err(FallbackError::RegistryError(e)),
            }
        }

        Err(FallbackError::ProviderNotFoundAnywhere {
            provider: provider.to_string(),
            namespaces: searched_namespaces,
        })
    }

    /// Get provider information with fallback
    pub async fn get_provider_info(
        &self,
        provider: &str,
        namespace: Option<&str>,
    ) -> Result<ProviderInfo, FallbackError> {
        let mut searched_namespaces = Vec::new();

        // First, try the specified namespace if provided
        if let Some(ns) = namespace {
            searched_namespaces.push(ns.to_string());
            match self.primary.get_provider_info(provider, ns).await {
                Ok(info) => {
                    logging::info(&format!(
                        "Found provider {} in specified namespace {}",
                        provider, ns
                    ));
                    return Ok(info);
                }
                Err(RegistryError::ProviderNotFound { .. }) => {
                    logging::debug(&format!(
                        "Provider {} not found in specified namespace {}, trying fallbacks",
                        provider, ns
                    ));
                }
                Err(e) => return Err(FallbackError::RegistryError(e)),
            }
        }

        // Try fallback namespaces
        for fallback_ns in &self.fallback_namespaces {
            // Skip if we already tried this namespace
            if namespace.is_some_and(|ns| ns == fallback_ns) {
                continue;
            }

            searched_namespaces.push(fallback_ns.clone());
            match self.primary.get_provider_info(provider, fallback_ns).await {
                Ok(info) => {
                    logging::info(&format!(
                        "Found provider {} in fallback namespace {}",
                        provider, fallback_ns
                    ));
                    return Ok(info);
                }
                Err(RegistryError::ProviderNotFound { .. }) => {
                    logging::debug(&format!(
                        "Provider {} not found in fallback namespace {}",
                        provider, fallback_ns
                    ));
                    continue;
                }
                Err(e) => return Err(FallbackError::RegistryError(e)),
            }
        }

        Err(FallbackError::ProviderNotFoundAnywhere {
            provider: provider.to_string(),
            namespaces: searched_namespaces,
        })
    }

    /// Search for provider documentation with fallback
    pub async fn search_docs_with_fallback(
        &self,
        provider: &str,
        namespace: Option<&str>,
        service_slug: &str,
        data_type: &str,
    ) -> Result<(Vec<crate::registry::client::DocIdResult>, String), FallbackError> {
        let mut searched_namespaces = Vec::new();

        // First, try the specified namespace if provided
        if let Some(ns) = namespace {
            searched_namespaces.push(ns.to_string());
            match self
                .primary
                .search_docs(provider, ns, service_slug, data_type)
                .await
            {
                Ok(docs) if !docs.is_empty() => {
                    logging::info(&format!(
                        "Found documentation for {} in specified namespace {}",
                        provider, ns
                    ));
                    return Ok((docs, ns.to_string()));
                }
                Ok(_) => {
                    logging::debug(&format!(
                        "No documentation found for {} in specified namespace {}, trying fallbacks",
                        provider, ns
                    ));
                }
                Err(e) => return Err(FallbackError::RegistryError(e)),
            }
        }

        // Try fallback namespaces
        for fallback_ns in &self.fallback_namespaces {
            // Skip if we already tried this namespace
            if namespace.is_some_and(|ns| ns == fallback_ns) {
                continue;
            }

            searched_namespaces.push(fallback_ns.clone());
            match self
                .primary
                .search_docs(provider, fallback_ns, service_slug, data_type)
                .await
            {
                Ok(docs) if !docs.is_empty() => {
                    logging::info(&format!(
                        "Found documentation for {} in fallback namespace {}",
                        provider, fallback_ns
                    ));
                    return Ok((docs, fallback_ns.clone()));
                }
                Ok(_) => {
                    logging::debug(&format!(
                        "No documentation found for {} in fallback namespace {}",
                        provider, fallback_ns
                    ));
                    continue;
                }
                Err(e) => return Err(FallbackError::RegistryError(e)),
            }
        }

        // If no docs found anywhere, return empty result with the first namespace attempted
        let used_namespace = namespace
            .unwrap_or(&self.fallback_namespaces[0])
            .to_string();

        Ok((vec![], used_namespace))
    }
}

impl Default for RegistryClientWithFallback {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fallback_client_creation() {
        let client = RegistryClientWithFallback::new();
        // Just test that client creates successfully
        assert_eq!(client.fallback_namespaces.len(), 3);
    }
}
