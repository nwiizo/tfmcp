use crate::registry::client::{ProviderInfo, RegistryClient, RegistryError};
use crate::shared::logging;
use futures::future::join_all;
use std::sync::Arc;
use std::time::Instant;

/// Batch fetcher for parallel provider operations
#[allow(dead_code)]
pub struct BatchFetcher {
    client: Arc<RegistryClient>,
    pub max_concurrent: usize,
}

impl BatchFetcher {
    pub fn new(client: Arc<RegistryClient>, max_concurrent: usize) -> Self {
        Self {
            client,
            max_concurrent: max_concurrent.clamp(1, 10), // Limit between 1-10
        }
    }

    /// Fetch multiple providers in parallel with controlled concurrency
    #[allow(dead_code)]
    pub async fn fetch_providers(
        &self,
        providers: Vec<(&str, &str)>, // (name, namespace) pairs
    ) -> Vec<Result<ProviderInfo, RegistryError>> {
        let start_time = Instant::now();
        let total_count = providers.len();
        
        logging::info(&format!(
            "Starting batch fetch for {} providers with max {} concurrent requests",
            total_count, self.max_concurrent
        ));

        let chunks: Vec<_> = providers.chunks(self.max_concurrent).collect();
        let mut all_results = Vec::new();

        for (chunk_index, chunk) in chunks.iter().enumerate() {
            logging::debug(&format!(
                "Processing chunk {}/{} with {} providers",
                chunk_index + 1, chunks.len(), chunk.len()
            ));

            let chunk_start = Instant::now();
            let futures: Vec<_> = chunk
                .iter()
                .map(|(name, namespace)| {
                    let client = self.client.clone();
                    let name = name.to_string();
                    let namespace = namespace.to_string();
                    
                    async move {
                        logging::debug(&format!("Fetching provider {}/{}", namespace, name));
                        let result = client.get_provider_info(&name, &namespace).await;
                        
                        match &result {
                            Ok(info) => {
                                logging::debug(&format!(
                                    "Successfully fetched {}/{} - {} downloads",
                                    namespace, name, info.downloads
                                ));
                            }
                            Err(e) => {
                                logging::warn(&format!(
                                    "Failed to fetch {}/{}: {}",
                                    namespace, name, e
                                ));
                            }
                        }
                        
                        result
                    }
                })
                .collect();

            let chunk_results = join_all(futures).await;
            all_results.extend(chunk_results);
            
            logging::debug(&format!(
                "Chunk {}/{} completed in {:?}",
                chunk_index + 1, chunks.len(), chunk_start.elapsed()
            ));
        }

        let total_duration = start_time.elapsed();
        let success_count = all_results.iter().filter(|r| r.is_ok()).count();
        
        logging::info(&format!(
            "Batch fetch completed: {}/{} successful in {:?} ({:.1} providers/sec)",
            success_count,
            total_count,
            total_duration,
            total_count as f64 / total_duration.as_secs_f64()
        ));

        all_results
    }

    /// Fetch multiple provider versions in parallel
    #[allow(dead_code)]
    pub async fn fetch_provider_versions(
        &self,
        providers: Vec<(&str, &str)>, // (name, namespace) pairs
    ) -> Vec<Result<(String, String), RegistryError>> {
        let start_time = Instant::now();
        let total_count = providers.len();
        
        logging::info(&format!(
            "Starting batch version fetch for {} providers",
            total_count
        ));

        let chunks: Vec<_> = providers.chunks(self.max_concurrent).collect();
        let mut all_results = Vec::new();

        for (chunk_index, chunk) in chunks.iter().enumerate() {
            let futures: Vec<_> = chunk
                .iter()
                .map(|(name, namespace)| {
                    let client = self.client.clone();
                    let name = name.to_string();
                    let namespace = namespace.to_string();
                    
                    async move {
                        let result = client.get_latest_version(&name, &namespace).await;
                        match &result {
                            Ok(version) => {
                                logging::debug(&format!(
                                    "Found version {} for {}/{}",
                                    version, namespace, name
                                ));
                                Ok((version.clone(), namespace))
                            }
                            Err(e) => {
                                logging::warn(&format!(
                                    "Failed to get version for {}/{}: {}",
                                    namespace, name, e
                                ));
                                Err(e.clone())
                            }
                        }
                    }
                })
                .collect();

            let chunk_results = join_all(futures).await;
            all_results.extend(chunk_results);
            
            logging::debug(&format!(
                "Version chunk {}/{} completed",
                chunk_index + 1, chunks.len()
            ));
        }

        let total_duration = start_time.elapsed();
        let success_count = all_results.iter().filter(|r| r.is_ok()).count();
        
        logging::info(&format!(
            "Batch version fetch completed: {}/{} successful in {:?}",
            success_count, total_count, total_duration
        ));

        all_results
    }

    /// Fetch documentation for multiple providers in parallel
    #[allow(dead_code)]
    pub async fn fetch_multiple_docs(
        &self,
        doc_requests: Vec<(&str, &str, &str, &str)>, // (provider, namespace, service, data_type)
    ) -> Vec<Result<Vec<crate::registry::client::DocIdResult>, RegistryError>> {
        let start_time = Instant::now();
        let total_count = doc_requests.len();
        
        logging::info(&format!(
            "Starting batch documentation search for {} requests",
            total_count
        ));

        let chunks: Vec<_> = doc_requests.chunks(self.max_concurrent).collect();
        let mut all_results = Vec::new();

        for (chunk_index, chunk) in chunks.iter().enumerate() {
            let futures: Vec<_> = chunk
                .iter()
                .map(|(provider, namespace, service, data_type)| {
                    let client = self.client.clone();
                    let provider = provider.to_string();
                    let namespace = namespace.to_string();
                    let service = service.to_string();
                    let data_type = data_type.to_string();
                    
                    async move {
                        logging::debug(&format!(
                            "Searching docs for {}/{} service {} type {}",
                            namespace, provider, service, data_type
                        ));
                        
                        let result = client.search_docs(&provider, &namespace, &service, &data_type).await;
                        
                        match &result {
                            Ok(docs) => {
                                logging::debug(&format!(
                                    "Found {} docs for {}/{} service {}",
                                    docs.len(), namespace, provider, service
                                ));
                            }
                            Err(e) => {
                                logging::warn(&format!(
                                    "Failed to search docs for {}/{} service {}: {}",
                                    namespace, provider, service, e
                                ));
                            }
                        }
                        
                        result
                    }
                })
                .collect();

            let chunk_results = join_all(futures).await;
            all_results.extend(chunk_results);
            
            logging::debug(&format!(
                "Documentation chunk {}/{} completed",
                chunk_index + 1, chunks.len()
            ));
        }

        let total_duration = start_time.elapsed();
        let success_count = all_results.iter().filter(|r| r.is_ok()).count();
        
        logging::info(&format!(
            "Batch documentation search completed: {}/{} successful in {:?}",
            success_count, total_count, total_duration
        ));

        all_results
    }
}

impl Default for BatchFetcher {
    fn default() -> Self {
        Self::new(Arc::new(RegistryClient::new()), 5)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_fetcher_creation() {
        let client = Arc::new(RegistryClient::new());
        let fetcher = BatchFetcher::new(client, 3);
        assert_eq!(fetcher.max_concurrent, 3);
    }

    #[test]
    fn test_batch_fetcher_max_concurrent_limits() {
        let client = Arc::new(RegistryClient::new());
        
        // Test upper limit
        let fetcher = BatchFetcher::new(client.clone(), 20);
        assert_eq!(fetcher.max_concurrent, 10);
        
        // Test lower limit
        let fetcher = BatchFetcher::new(client.clone(), 0);
        assert_eq!(fetcher.max_concurrent, 1);
        
        // Test normal case
        let fetcher = BatchFetcher::new(client, 5);
        assert_eq!(fetcher.max_concurrent, 5);
    }

    #[tokio::test]
    async fn test_empty_batch_fetch() {
        let fetcher = BatchFetcher::default();
        let results = fetcher.fetch_providers(vec![]).await;
        assert!(results.is_empty());
    }
}