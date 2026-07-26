use crate::registry::client::{ProviderInfo, RegistryClient, RegistryError};
use crate::shared::logging;
use futures::future::BoxFuture;
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
    pub primary: Arc<RegistryClient>,
    pub fallback_namespaces: Vec<String>,
}

#[derive(Clone, Copy)]
enum NamespaceAttemptKind {
    Specified,
    Fallback,
}

struct NamespaceAttempt {
    namespace: String,
    kind: NamespaceAttemptKind,
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
        self.find_provider_with_fallback(
            provider,
            namespace,
            lookup_latest_version,
            |provider, namespace, kind, version| {
                format!(
                    "Found provider {} in {} namespace {} with version {}",
                    provider,
                    namespace_kind_label(kind),
                    namespace,
                    version
                )
            },
        )
        .await
    }

    /// Get provider information with fallback
    pub async fn get_provider_info(
        &self,
        provider: &str,
        namespace: Option<&str>,
    ) -> Result<ProviderInfo, FallbackError> {
        let (info, _) = self
            .find_provider_with_fallback(
                provider,
                namespace,
                lookup_provider_info,
                |provider, namespace, kind, _| {
                    format!(
                        "Found provider {} in {} namespace {}",
                        provider,
                        namespace_kind_label(kind),
                        namespace
                    )
                },
            )
            .await?;
        Ok(info)
    }

    async fn find_provider_with_fallback<T, FormatFound>(
        &self,
        provider: &str,
        namespace: Option<&str>,
        lookup: for<'a> fn(
            &'a RegistryClient,
            &'a str,
            &'a str,
        ) -> BoxFuture<'a, Result<T, RegistryError>>,
        format_found: FormatFound,
    ) -> Result<(T, String), FallbackError>
    where
        FormatFound: Fn(&str, &str, NamespaceAttemptKind, &T) -> String,
    {
        let mut searched_namespaces = Vec::new();

        for attempt in self.namespace_attempts(namespace) {
            searched_namespaces.push(attempt.namespace.clone());
            match lookup(&self.primary, provider, &attempt.namespace).await {
                Ok(result) => {
                    logging::info(&format_found(
                        provider,
                        &attempt.namespace,
                        attempt.kind,
                        &result,
                    ));
                    return Ok((result, attempt.namespace));
                }
                Err(RegistryError::ProviderNotFound { .. }) => {
                    log_provider_not_found(provider, &attempt);
                }
                Err(e) => return Err(FallbackError::RegistryError(e)),
            }
        }

        Err(FallbackError::ProviderNotFoundAnywhere {
            provider: provider.to_string(),
            namespaces: searched_namespaces,
        })
    }

    fn namespace_attempts(&self, namespace: Option<&str>) -> Vec<NamespaceAttempt> {
        let mut attempts = Vec::new();

        if let Some(ns) = namespace {
            attempts.push(NamespaceAttempt {
                namespace: ns.to_string(),
                kind: NamespaceAttemptKind::Specified,
            });
        }

        attempts.extend(
            self.fallback_namespaces
                .iter()
                .filter(|fallback_ns| namespace.is_none_or(|ns| ns != fallback_ns.as_str()))
                .cloned()
                .map(|namespace| NamespaceAttempt {
                    namespace,
                    kind: NamespaceAttemptKind::Fallback,
                }),
        );

        attempts
    }

    /// Search for provider documentation with fallback
    #[allow(dead_code)]
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
                        "Found documentation for {provider} in specified namespace {ns}"
                    ));
                    return Ok((docs, ns.to_string()));
                }
                Ok(_) => {
                    logging::debug(&format!(
                        "No documentation found for {provider} in specified namespace {ns}, trying fallbacks"
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
                        "Found documentation for {provider} in fallback namespace {fallback_ns}"
                    ));
                    return Ok((docs, fallback_ns.clone()));
                }
                Ok(_) => {
                    logging::debug(&format!(
                        "No documentation found for {provider} in fallback namespace {fallback_ns}"
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

fn lookup_latest_version<'a>(
    client: &'a RegistryClient,
    provider: &'a str,
    namespace: &'a str,
) -> BoxFuture<'a, Result<String, RegistryError>> {
    Box::pin(client.get_latest_version(provider, namespace))
}

fn lookup_provider_info<'a>(
    client: &'a RegistryClient,
    provider: &'a str,
    namespace: &'a str,
) -> BoxFuture<'a, Result<ProviderInfo, RegistryError>> {
    Box::pin(client.get_provider_info(provider, namespace))
}

fn namespace_kind_label(kind: NamespaceAttemptKind) -> &'static str {
    match kind {
        NamespaceAttemptKind::Specified => "specified",
        NamespaceAttemptKind::Fallback => "fallback",
    }
}

fn log_provider_not_found(provider: &str, attempt: &NamespaceAttempt) {
    match attempt.kind {
        NamespaceAttemptKind::Specified => {
            logging::debug(&format!(
                "Provider {} not found in specified namespace {}, trying fallbacks",
                provider, attempt.namespace
            ));
        }
        NamespaceAttemptKind::Fallback => {
            logging::debug(&format!(
                "Provider {} not found in fallback namespace {}",
                provider, attempt.namespace
            ));
        }
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
