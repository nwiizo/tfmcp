use crate::registry::client::{ProviderInfo, RegistryClient, RegistryError};
use crate::shared::logging;
use futures::future::join_all;
use std::future::Future;
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
        self.fetch_provider_batch(providers, ProviderBatchKind::Info, fetch_provider_info)
            .await
    }

    /// Fetch multiple provider versions in parallel
    #[allow(dead_code)]
    pub async fn fetch_provider_versions(
        &self,
        providers: Vec<(&str, &str)>, // (name, namespace) pairs
    ) -> Vec<Result<(String, String), RegistryError>> {
        self.fetch_provider_batch(
            providers,
            ProviderBatchKind::Version,
            fetch_provider_version,
        )
        .await
    }

    async fn fetch_provider_batch<T, Fetch, Fut>(
        &self,
        providers: Vec<(&str, &str)>,
        kind: ProviderBatchKind,
        fetch: Fetch,
    ) -> Vec<Result<T, RegistryError>>
    where
        Fetch: Fn(Arc<RegistryClient>, String, String) -> Fut,
        Fut: Future<Output = Result<T, RegistryError>>,
    {
        let start_time = Instant::now();
        let total_count = providers.len();
        kind.log_start(total_count, self.max_concurrent);

        let results = self
            .fetch_provider_chunks(
                providers,
                kind.chunk_label(),
                kind.include_chunk_size_and_duration(),
                fetch,
            )
            .await;
        kind.log_completion(total_count, start_time, &results);
        results
    }

    async fn fetch_provider_chunks<T, Fetch, Fut>(
        &self,
        providers: Vec<(&str, &str)>,
        chunk_label: &str,
        include_size_and_duration: bool,
        fetch: Fetch,
    ) -> Vec<Result<T, RegistryError>>
    where
        Fetch: Fn(Arc<RegistryClient>, String, String) -> Fut,
        Fut: Future<Output = Result<T, RegistryError>>,
    {
        let chunks: Vec<_> = providers.chunks(self.max_concurrent).collect();
        let mut all_results = Vec::new();

        for (chunk_index, chunk) in chunks.iter().enumerate() {
            if include_size_and_duration {
                logging::debug(&format!(
                    "Processing chunk {}/{} with {} providers",
                    chunk_index + 1,
                    chunks.len(),
                    chunk.len()
                ));
            }

            let chunk_start = Instant::now();
            let futures: Vec<_> = chunk
                .iter()
                .map(|(name, namespace)| {
                    fetch(self.client.clone(), name.to_string(), namespace.to_string())
                })
                .collect();

            let chunk_results = join_all(futures).await;
            all_results.extend(chunk_results);

            log_chunk_completion(
                chunk_label,
                chunk_index + 1,
                chunks.len(),
                include_size_and_duration.then(|| chunk_start.elapsed()),
            );
        }

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
            "Starting batch documentation search for {total_count} requests"
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
                            "Searching docs for {namespace}/{provider} service {service} type {data_type}"
                        ));

                        let result = client
                            .search_docs(&provider, &namespace, &service, &data_type)
                            .await;

                        match &result {
                            Ok(docs) => {
                                logging::debug(&format!(
                                    "Found {} docs for {}/{} service {}",
                                    docs.len(),
                                    namespace,
                                    provider,
                                    service
                                ));
                            }
                            Err(e) => {
                                logging::warn(&format!(
                                    "Failed to search docs for {namespace}/{provider} service {service}: {e}"
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
                chunk_index + 1,
                chunks.len()
            ));
        }

        let total_duration = start_time.elapsed();
        let success_count = all_results.iter().filter(|r| r.is_ok()).count();

        logging::info(&format!(
            "Batch documentation search completed: {success_count}/{total_count} successful in {total_duration:?}"
        ));

        all_results
    }
}

#[derive(Clone, Copy)]
enum ProviderBatchKind {
    Info,
    Version,
}

impl ProviderBatchKind {
    fn chunk_label(&self) -> &'static str {
        match self {
            Self::Info => "Chunk",
            Self::Version => "Version chunk",
        }
    }

    fn include_chunk_size_and_duration(&self) -> bool {
        matches!(self, Self::Info)
    }

    fn log_start(&self, total_count: usize, max_concurrent: usize) {
        match self {
            Self::Info => logging::info(&format!(
                "Starting batch fetch for {total_count} providers with max {max_concurrent} concurrent requests"
            )),
            Self::Version => logging::info(&format!(
                "Starting batch version fetch for {total_count} providers"
            )),
        }
    }

    fn log_completion<T>(
        &self,
        total_count: usize,
        start_time: Instant,
        results: &[Result<T, RegistryError>],
    ) {
        match self {
            Self::Info => {
                log_batch_completion("Batch fetch", total_count, start_time, results, true)
            }
            Self::Version => log_batch_completion(
                "Batch version fetch",
                total_count,
                start_time,
                results,
                false,
            ),
        }
    }
}

async fn fetch_provider_info(
    client: Arc<RegistryClient>,
    name: String,
    namespace: String,
) -> Result<ProviderInfo, RegistryError> {
    logging::debug(&format!("Fetching provider {namespace}/{name}"));
    let result = client.get_provider_info(&name, &namespace).await;

    match &result {
        Ok(info) => {
            logging::debug(&format!(
                "Successfully fetched {}/{} - {} downloads",
                namespace, name, info.downloads
            ));
        }
        Err(e) => {
            logging::warn(&format!("Failed to fetch {namespace}/{name}: {e}"));
        }
    }

    result
}

async fn fetch_provider_version(
    client: Arc<RegistryClient>,
    name: String,
    namespace: String,
) -> Result<(String, String), RegistryError> {
    let result = client.get_latest_version(&name, &namespace).await;

    match &result {
        Ok(version) => {
            logging::debug(&format!("Found version {version} for {namespace}/{name}"));
            Ok((version.clone(), namespace))
        }
        Err(e) => {
            logging::warn(&format!(
                "Failed to get version for {namespace}/{name}: {e}"
            ));
            Err(e.clone())
        }
    }
}

fn log_chunk_completion(
    label: &str,
    chunk_number: usize,
    chunk_count: usize,
    elapsed: Option<std::time::Duration>,
) {
    if let Some(elapsed) = elapsed {
        logging::debug(&format!(
            "{label} {chunk_number}/{chunk_count} completed in {elapsed:?}"
        ));
    } else {
        logging::debug(&format!("{label} {chunk_number}/{chunk_count} completed"));
    }
}

fn log_batch_completion<T>(
    label: &str,
    total_count: usize,
    start_time: Instant,
    results: &[Result<T, RegistryError>],
    include_rate: bool,
) {
    let total_duration = start_time.elapsed();
    let success_count = results.iter().filter(|r| r.is_ok()).count();

    if include_rate {
        logging::info(&format!(
            "{} completed: {}/{} successful in {:?} ({:.1} providers/sec)",
            label,
            success_count,
            total_count,
            total_duration,
            total_count as f64 / total_duration.as_secs_f64()
        ));
    } else {
        logging::info(&format!(
            "{label} completed: {success_count}/{total_count} successful in {total_duration:?}"
        ));
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
