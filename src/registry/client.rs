use reqwest::{Client, RequestBuilder, StatusCode};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

pub use crate::registry::types::*;

pub struct RegistryClient {
    client: Client,
    base_url: String,
}

fn value_str(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

fn value_str_or(value: &Value, key: &str, fallback: &str) -> String {
    value
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or(fallback)
        .to_string()
}

fn value_opt_str(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn provider_info_from_value(provider: &Value) -> Option<ProviderInfo> {
    let name = provider.get("name").and_then(|v| v.as_str())?;
    Some(ProviderInfo {
        name: name.to_string(),
        namespace: value_str(provider, "namespace"),
        description: value_str(provider, "description"),
        downloads: provider
            .get("downloads")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        version: value_str(provider, "version"),
        ..Default::default()
    })
}

fn doc_id_from_value(doc: &Value) -> Option<DocIdResult> {
    let doc_result = DocIdResult {
        id: value_str(doc, "id"),
        title: value_str(doc, "title"),
        description: value_str(doc, "description"),
        category: value_str(doc, "category"),
        slug: value_opt_str(doc, "slug"),
        path: value_opt_str(doc, "path"),
        subcategory: value_opt_str(doc, "subcategory"),
        extra: HashMap::new(),
    };

    (!doc_result.id.is_empty() || !doc_result.title.is_empty()).then_some(doc_result)
}

fn module_info_from_value(module: &Value) -> Option<ModuleInfo> {
    let id = value_str(module, "id");
    (!id.is_empty()).then(|| ModuleInfo {
        id,
        namespace: value_str(module, "namespace"),
        name: value_str(module, "name"),
        provider: value_str(module, "provider"),
        version: value_str(module, "version"),
        description: value_str(module, "description"),
        source: value_str(module, "source"),
        published_at: value_str(module, "published_at"),
        downloads: module
            .get("downloads")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        verified: module
            .get("verified")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        owner: value_str(module, "owner"),
        extra: HashMap::new(),
    })
}

fn module_versions_from_response(response: ModuleVersionsResponse) -> Vec<String> {
    response
        .modules
        .into_iter()
        .flat_map(|module| module.versions.into_iter().map(|version| version.version))
        .filter(|version| !version.is_empty())
        .collect()
}

fn module_versions_from_fallback_json(json_value: &Value) -> Option<Vec<String>> {
    let versions: Vec<String> = json_value
        .get("modules")?
        .as_array()?
        .iter()
        .filter_map(|module| module.get("versions").and_then(|v| v.as_array()))
        .flatten()
        .filter_map(|version| version.get("version").and_then(|v| v.as_str()))
        .map(|version| version.to_string())
        .collect();

    (!versions.is_empty()).then_some(versions)
}

fn parse_provider_info_json(
    json_value: Value,
    provider_name: &str,
    namespace: &str,
) -> ProviderInfo {
    if let Ok(provider_info) = serde_json::from_value::<ProviderInfo>(json_value.clone()) {
        info!(
            "Successfully retrieved provider info for {}/{}",
            namespace, provider_name
        );
        return provider_info;
    }

    provider_info_fallback(json_value, provider_name, namespace)
}

fn provider_info_fallback(json_value: Value, provider_name: &str, namespace: &str) -> ProviderInfo {
    error!(
        "Parsed JSON was: {}",
        serde_json::to_string_pretty(&json_value).unwrap_or_else(|_| "Invalid JSON".to_string())
    );
    warn!("Using fallback provider info parsing due to deserialization error");
    ProviderInfo {
        name: value_str_or(&json_value, "name", provider_name),
        namespace: value_str_or(&json_value, "namespace", namespace),
        description: value_str(&json_value, "description"),
        downloads: json_value
            .get("downloads")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        ..Default::default()
    }
}

fn parse_latest_provider_version_json(
    json_value: Value,
    provider_name: &str,
    namespace: &str,
) -> Result<String, RegistryError> {
    match serde_json::from_value::<ProviderVersions>(json_value.clone()) {
        Ok(mut versions) => {
            if versions.versions.is_empty()
                && let Some(data) = versions.data.as_ref()
            {
                versions.versions = data
                    .iter()
                    .map(|v| v.version.clone())
                    .filter(|v| !v.is_empty())
                    .collect();
            }

            provider_latest_version(versions.versions, provider_name, namespace)
        }
        Err(e) => {
            error!("Failed to deserialize ProviderVersions: {}", e);
            if let Some(first_version) = json_value
                .get("versions")
                .and_then(|v| v.as_array())
                .and_then(|versions| versions.first())
                .and_then(|v| v.as_str())
            {
                warn!("Using fallback version parsing");
                return Ok(first_version.to_string());
            }

            Err(RegistryError::JsonError(format!(
                "Failed to parse versions: {e}"
            )))
        }
    }
}

fn provider_latest_version(
    versions: Vec<String>,
    provider_name: &str,
    namespace: &str,
) -> Result<String, RegistryError> {
    if versions.is_empty() {
        warn!(
            "No versions available for provider {}/{}",
            namespace, provider_name
        );
        return Err(RegistryError::NoVersionsAvailable {
            provider: provider_name.to_string(),
            namespace: namespace.to_string(),
        });
    }

    let latest_version = versions
        .last()
        .cloned()
        .ok_or(RegistryError::InvalidResponse)?;
    info!(
        "Found latest version {} for provider {}/{}",
        latest_version, namespace, provider_name
    );
    Ok(latest_version)
}

enum RegistryNotFound<'a> {
    Provider {
        provider: &'a str,
        namespace: &'a str,
        label: &'static str,
    },
    Module {
        module: &'a str,
        provider: &'a str,
        namespace: &'a str,
        label: &'static str,
    },
}

impl RegistryNotFound<'_> {
    fn warn_message(&self) -> String {
        match self {
            Self::Provider {
                provider,
                namespace,
                label,
            } => {
                format!("{label} not found: {namespace}/{provider}")
            }
            Self::Module {
                module,
                provider,
                namespace,
                label,
            } => {
                format!("{label} not found: {namespace}/{module}/{provider}")
            }
        }
    }

    fn error(&self) -> RegistryError {
        match self {
            Self::Provider {
                provider,
                namespace,
                ..
            } => RegistryError::ProviderNotFound {
                provider: (*provider).to_string(),
                namespace: (*namespace).to_string(),
            },
            Self::Module {
                module,
                provider,
                namespace,
                ..
            } => RegistryError::ModuleNotFound {
                module: (*module).to_string(),
                provider: (*provider).to_string(),
                namespace: (*namespace).to_string(),
            },
        }
    }
}

struct JsonRequestContext<'a> {
    label: &'static str,
    preview_chars: usize,
    not_found: Option<RegistryNotFound<'a>>,
}

enum RegistrySearchKind {
    Provider,
    Module,
}

impl RegistrySearchKind {
    fn url(&self, base_url: &str) -> String {
        match self {
            Self::Provider => format!("{base_url}/v1/providers"),
            Self::Module => format!("{base_url}/v1/modules/search"),
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Self::Provider => "provider search",
            Self::Module => "module search",
        }
    }

    fn params<'a>(&self, query: &'a str) -> Vec<(&'static str, &'a str)> {
        match self {
            Self::Provider => vec![("q", query)],
            Self::Module => vec![("q", query), ("limit", "20")],
        }
    }
}

enum ProviderEndpoint {
    Info,
    Versions,
}

impl ProviderEndpoint {
    fn url(&self, base_url: &str, namespace: &str, provider_name: &str) -> String {
        match self {
            Self::Info => format!("{base_url}/v1/providers/{namespace}/{provider_name}"),
            Self::Versions => {
                format!("{base_url}/v1/providers/{namespace}/{provider_name}/versions")
            }
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Self::Info => "provider info",
            Self::Versions => "provider versions",
        }
    }

    fn preview_chars(&self) -> usize {
        match self {
            Self::Info => 1000,
            Self::Versions => 500,
        }
    }

    fn not_found_label(&self) -> &'static str {
        match self {
            Self::Info => "Provider",
            Self::Versions => "Provider versions",
        }
    }
}

trait RegistrySearchItem: Sized {
    type Response: DeserializeOwned;

    fn items(response: Self::Response) -> Vec<Self>;
    fn fallback(client: &RegistryClient, json_value: &Value) -> Vec<Self>;
    fn parse_error_label() -> &'static str;
    fn fallback_warning() -> &'static str;
}

impl RegistrySearchItem for ProviderInfo {
    type Response = RegistrySearchResponse;

    fn items(mut response: Self::Response) -> Vec<Self> {
        if response.providers.is_empty()
            && let Some(data) = response.data.take()
        {
            response.providers = data;
        }
        response.providers
    }

    fn fallback(client: &RegistryClient, json_value: &Value) -> Vec<Self> {
        json_value
            .get("providers")
            .and_then(|v| v.as_array())
            .map(|providers| client.extract_providers_from_array(providers))
            .unwrap_or_default()
    }

    fn parse_error_label() -> &'static str {
        "search response"
    }

    fn fallback_warning() -> &'static str {
        "Using fallback provider search parsing"
    }
}

impl RegistrySearchItem for ModuleInfo {
    type Response = ModuleSearchResponse;

    fn items(response: Self::Response) -> Vec<Self> {
        response.modules
    }

    fn fallback(client: &RegistryClient, json_value: &Value) -> Vec<Self> {
        json_value
            .get("modules")
            .and_then(|v| v.as_array())
            .map(|modules| client.extract_modules_from_array(modules))
            .unwrap_or_default()
    }

    fn parse_error_label() -> &'static str {
        "module search response"
    }

    fn fallback_warning() -> &'static str {
        "Using fallback module search parsing"
    }
}

impl Default for RegistryClient {
    fn default() -> Self {
        Self::new()
    }
}

impl RegistryClient {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("tfmcp/0.1.3")
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_else(|_| Client::new()), // Fallback to default client
            base_url: "https://registry.terraform.io".to_string(),
        }
    }

    async fn send_json_request(
        &self,
        request: RequestBuilder,
        context: JsonRequestContext<'_>,
    ) -> Result<Value, RegistryError> {
        let response = request.send().await?;
        let status = response.status();

        debug!("{} response status: {}", context.label, status);

        if status == StatusCode::NOT_FOUND
            && let Some(not_found) = context.not_found
        {
            warn!("{}", not_found.warn_message());
            return Err(not_found.error());
        }

        if status == StatusCode::TOO_MANY_REQUESTS {
            warn!("Rate limit exceeded for {}", context.label);
            return Err(RegistryError::RateLimited);
        }

        if !status.is_success() {
            error!("HTTP error {} for {}", status, context.label);
            return Err(RegistryError::HttpError(format!("HTTP {status}")));
        }

        let response_text = response.text().await?;
        debug!(
            "{} response (first {} chars): {}",
            context.label,
            context.preview_chars,
            &response_text
                .chars()
                .take(context.preview_chars)
                .collect::<String>()
        );

        serde_json::from_str::<Value>(&response_text).map_err(|e| {
            error!("Failed to parse {} JSON: {}", context.label, e);
            error!("Response text was: {}", response_text);
            RegistryError::JsonError(format!("Invalid JSON response: {e}"))
        })
    }

    async fn search_registry_items<T>(
        &self,
        kind: RegistrySearchKind,
        query: &str,
    ) -> Result<Vec<T>, RegistryError>
    where
        T: RegistrySearchItem,
    {
        let url = kind.url(&self.base_url);
        let label = kind.label();
        debug!("Searching {} with query '{}' at URL: {}", label, query, url);
        let json_value = self
            .send_json_request(
                self.client.get(&url).query(&kind.params(query)),
                JsonRequestContext {
                    label,
                    preview_chars: 1000,
                    not_found: None,
                },
            )
            .await?;

        let items = match serde_json::from_value::<T::Response>(json_value.clone()) {
            Ok(response) => T::items(response),
            Err(e) => {
                error!("Failed to deserialize {}: {}", T::parse_error_label(), e);
                let fallback = T::fallback(self, &json_value);
                if !fallback.is_empty() {
                    warn!("{}", T::fallback_warning());
                    return Ok(fallback);
                }
                return Err(RegistryError::JsonError(format!(
                    "Failed to parse {}: {}",
                    T::parse_error_label(),
                    e
                )));
            }
        };

        if items.is_empty() {
            info!("No {} results found for query: {}", label, query);
            return Err(RegistryError::NoSearchResults {
                query: query.to_string(),
            });
        }

        info!(
            "Found {} {} results for query: {}",
            items.len(),
            label,
            query
        );
        Ok(items)
    }

    async fn provider_endpoint_json(
        &self,
        endpoint: ProviderEndpoint,
        provider_name: &str,
        namespace: &str,
    ) -> Result<Value, RegistryError> {
        let url = endpoint.url(&self.base_url, namespace, provider_name);
        debug!("Fetching {} from URL: {}", endpoint.label(), url);
        self.send_json_request(
            self.client.get(&url),
            JsonRequestContext {
                label: endpoint.label(),
                preview_chars: endpoint.preview_chars(),
                not_found: Some(RegistryNotFound::Provider {
                    provider: provider_name,
                    namespace,
                    label: endpoint.not_found_label(),
                }),
            },
        )
        .await
    }

    /// Search for providers in the Terraform Registry with improved error handling
    pub async fn search_providers(&self, query: &str) -> Result<Vec<ProviderInfo>, RegistryError> {
        self.search_registry_items(RegistrySearchKind::Provider, query)
            .await
    }

    /// Helper function to extract providers from JSON array with fallback parsing
    fn extract_providers_from_array(&self, providers_array: &[Value]) -> Vec<ProviderInfo> {
        providers_array
            .iter()
            .filter_map(provider_info_from_value)
            .collect()
    }

    /// Get provider information by namespace and name with detailed error logging
    pub async fn get_provider_info(
        &self,
        provider_name: &str,
        namespace: &str,
    ) -> Result<ProviderInfo, RegistryError> {
        let json_value = self
            .provider_endpoint_json(ProviderEndpoint::Info, provider_name, namespace)
            .await?;
        debug!("Successfully parsed JSON. Structure: {:#?}", json_value);
        Ok(parse_provider_info_json(
            json_value,
            provider_name,
            namespace,
        ))
    }

    /// Get latest version of a provider with improved error handling
    pub async fn get_latest_version(
        &self,
        provider_name: &str,
        namespace: &str,
    ) -> Result<String, RegistryError> {
        let json_value = self
            .provider_endpoint_json(ProviderEndpoint::Versions, provider_name, namespace)
            .await?;
        debug!("Parsed versions JSON structure: {:#?}", json_value);
        parse_latest_provider_version_json(json_value, provider_name, namespace)
    }

    /// Search for provider documentation IDs with multiple endpoint patterns
    pub async fn search_docs(
        &self,
        provider_name: &str,
        namespace: &str,
        service_slug: &str,
        data_type: &str,
    ) -> Result<Vec<DocIdResult>, RegistryError> {
        debug!(
            "Searching docs for provider: {}/{}, service: {}, type: {}",
            namespace, provider_name, service_slug, data_type
        );

        // Try multiple URL patterns as the API endpoint may vary
        let url_patterns = [
            format!(
                "{}/v1/providers/{}/{}/docs",
                self.base_url, namespace, provider_name
            ),
            format!(
                "{}/v2/providers/{}/{}/docs",
                self.base_url, namespace, provider_name
            ),
            format!(
                "{}/providers/{}/{}/docs",
                self.base_url, namespace, provider_name
            ),
            format!(
                "{}/docs/providers/{}/{}",
                self.base_url, namespace, provider_name
            ),
        ];

        let query_params = [
            vec![("category", data_type), ("slug", service_slug)],
            vec![("type", data_type), ("slug", service_slug)],
            vec![
                ("filter[category]", data_type),
                ("filter[slug]", service_slug),
            ],
            vec![("q", service_slug), ("category", data_type)],
        ];

        for (url_idx, url) in url_patterns.iter().enumerate() {
            for params in query_params.iter() {
                debug!(
                    "Trying URL pattern {}/{}: {} with params: {:?}",
                    url_idx + 1,
                    url_patterns.len(),
                    url,
                    params
                );

                let response = self.client.get(url).query(params).send().await?;
                let status = response.status();

                debug!("Response status: {} for URL: {}", status, url);

                if status == 429 {
                    warn!("Rate limit exceeded for docs search");
                    return Err(RegistryError::RateLimited);
                }

                if status == 404 {
                    debug!(
                        "404 for pattern {}/{}, trying next pattern",
                        url_idx + 1,
                        url_patterns.len()
                    );
                    continue;
                }

                if !status.is_success() {
                    warn!("HTTP error {} for docs URL: {}", status, url);
                    continue;
                }

                let response_text = response.text().await?;
                debug!(
                    "Docs response (first 500 chars): {}",
                    &response_text.chars().take(500).collect::<String>()
                );

                match serde_json::from_str::<Value>(&response_text) {
                    Ok(json_value) => {
                        debug!("Parsed docs JSON structure: {:#?}", json_value);

                        // Try to deserialize into ProviderDocsResponse
                        match serde_json::from_value::<ProviderDocsResponse>(json_value.clone()) {
                            Ok(mut docs_response) => {
                                // Handle multiple response format possibilities
                                if docs_response.data.is_empty() {
                                    if let Some(docs) = docs_response.docs.take() {
                                        docs_response.data = docs;
                                    } else if let Some(documentation) =
                                        docs_response.documentation.take()
                                    {
                                        docs_response.data = documentation;
                                    }
                                }

                                if !docs_response.data.is_empty() {
                                    info!(
                                        "Found {} docs for {}/{} service: {}",
                                        docs_response.data.len(),
                                        namespace,
                                        provider_name,
                                        service_slug
                                    );
                                    return Ok(docs_response.data);
                                }
                            }
                            Err(e) => {
                                warn!("Failed to deserialize docs response: {}", e);

                                // Try manual extraction from various JSON structures
                                if let Some(docs_array) =
                                    json_value.get("data").and_then(|v| v.as_array())
                                {
                                    let docs = self.extract_docs_from_array(docs_array);
                                    if !docs.is_empty() {
                                        info!(
                                            "Extracted {} docs using fallback parsing",
                                            docs.len()
                                        );
                                        return Ok(docs);
                                    }
                                }

                                if let Some(docs_array) =
                                    json_value.get("docs").and_then(|v| v.as_array())
                                {
                                    let docs = self.extract_docs_from_array(docs_array);
                                    if !docs.is_empty() {
                                        info!(
                                            "Extracted {} docs using fallback parsing (docs field)",
                                            docs.len()
                                        );
                                        return Ok(docs);
                                    }
                                }

                                // Try direct array
                                if let Some(docs_array) = json_value.as_array() {
                                    let docs = self.extract_docs_from_array(docs_array);
                                    if !docs.is_empty() {
                                        info!(
                                            "Extracted {} docs using fallback parsing (direct array)",
                                            docs.len()
                                        );
                                        return Ok(docs);
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to parse docs JSON: {}", e);
                        continue;
                    }
                }
            }
        }

        warn!(
            "No documentation found for {}/{} service: {} after trying all patterns",
            namespace, provider_name, service_slug
        );
        Ok(vec![])
    }

    /// Helper function to extract docs from JSON array with fallback parsing
    fn extract_docs_from_array(&self, docs_array: &[Value]) -> Vec<DocIdResult> {
        docs_array.iter().filter_map(doc_id_from_value).collect()
    }

    /// Get provider documentation content by ID with multiple endpoint patterns
    pub async fn get_doc_content(&self, doc_id: &str) -> Result<String, RegistryError> {
        debug!("Fetching documentation content for ID: {}", doc_id);

        // Try multiple URL patterns for documentation content
        let url_patterns = [
            format!("{}/v1/docs/{}", self.base_url, doc_id),
            format!("{}/v2/docs/{}", self.base_url, doc_id),
            format!("{}/docs/{}", self.base_url, doc_id),
            format!("{}/documentation/{}", self.base_url, doc_id),
        ];

        for (idx, url) in url_patterns.iter().enumerate() {
            debug!(
                "Trying documentation URL pattern {}/{}: {}",
                idx + 1,
                url_patterns.len(),
                url
            );

            let response = self.client.get(url).send().await?;
            let status = response.status();

            debug!("Response status: {} for docs URL: {}", status, url);

            if status == 429 {
                warn!("Rate limit exceeded for documentation content");
                return Err(RegistryError::RateLimited);
            }

            if status == 404 {
                debug!(
                    "404 for docs pattern {}/{}, trying next pattern",
                    idx + 1,
                    url_patterns.len()
                );
                continue;
            }

            if !status.is_success() {
                warn!("HTTP error {} for docs content URL: {}", status, url);
                continue;
            }

            let content = response.text().await?;
            debug!(
                "Retrieved documentation content ({} chars) for ID: {}",
                content.len(),
                doc_id
            );

            if !content.trim().is_empty() {
                info!(
                    "Successfully retrieved documentation content for ID: {}",
                    doc_id
                );
                return Ok(content);
            }
        }

        error!(
            "Documentation not found for ID: {} after trying all patterns",
            doc_id
        );
        Err(RegistryError::DocumentationNotFound {
            doc_id: doc_id.to_string(),
        })
    }

    // ==================== Module API Methods ====================

    /// Search for modules in the Terraform Registry
    pub async fn search_modules(&self, query: &str) -> Result<Vec<ModuleInfo>, RegistryError> {
        self.search_registry_items(RegistrySearchKind::Module, query)
            .await
    }

    /// Helper function to extract modules from JSON array
    fn extract_modules_from_array(&self, modules_array: &[Value]) -> Vec<ModuleInfo> {
        modules_array
            .iter()
            .filter_map(module_info_from_value)
            .collect()
    }

    /// Get module details by namespace, name, and provider
    pub async fn get_module_details(
        &self,
        namespace: &str,
        name: &str,
        provider: &str,
        version: Option<&str>,
    ) -> Result<ModuleDetails, RegistryError> {
        let url = match version {
            Some(ver) => format!(
                "{}/v1/modules/{}/{}/{}/{}",
                self.base_url, namespace, name, provider, ver
            ),
            None => format!(
                "{}/v1/modules/{}/{}/{}",
                self.base_url, namespace, name, provider
            ),
        };

        debug!("Fetching module details from URL: {}", url);

        let json_value = self
            .send_json_request(
                self.client.get(&url),
                JsonRequestContext {
                    label: "module details",
                    preview_chars: 1000,
                    not_found: Some(RegistryNotFound::Module {
                        module: name,
                        provider,
                        namespace,
                        label: "Module",
                    }),
                },
            )
            .await?;

        match serde_json::from_value::<ModuleDetails>(json_value.clone()) {
            Ok(module_details) => {
                info!(
                    "Successfully retrieved module details for {}/{}/{}",
                    namespace, name, provider
                );
                Ok(module_details)
            }
            Err(e) => {
                error!("Failed to deserialize module details: {}", e);
                warn!("Using fallback module details parsing");
                Ok(ModuleDetails {
                    id: value_str(&json_value, "id"),
                    namespace: value_str_or(&json_value, "namespace", namespace),
                    name: value_str_or(&json_value, "name", name),
                    provider: value_str_or(&json_value, "provider", provider),
                    version: value_str(&json_value, "version"),
                    description: value_str(&json_value, "description"),
                    source: value_str(&json_value, "source"),
                    published_at: value_str(&json_value, "published_at"),
                    downloads: json_value
                        .get("downloads")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0),
                    verified: json_value
                        .get("verified")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    root: None,
                    submodules: vec![],
                    versions: vec![],
                    extra: HashMap::new(),
                })
            }
        }
    }

    /// Get all versions available for a module
    pub async fn get_module_versions(
        &self,
        namespace: &str,
        name: &str,
        provider: &str,
    ) -> Result<Vec<String>, RegistryError> {
        let url = format!(
            "{}/v1/modules/{}/{}/{}/versions",
            self.base_url, namespace, name, provider
        );

        debug!("Fetching module versions from URL: {}", url);

        let json_value = self
            .send_json_request(
                self.client.get(&url),
                JsonRequestContext {
                    label: "module versions",
                    preview_chars: 500,
                    not_found: Some(RegistryNotFound::Module {
                        module: name,
                        provider,
                        namespace,
                        label: "Module versions",
                    }),
                },
            )
            .await?;

        if let Ok(versions_response) =
            serde_json::from_value::<ModuleVersionsResponse>(json_value.clone())
        {
            let versions = module_versions_from_response(versions_response);
            if !versions.is_empty() {
                info!(
                    "Found {} versions for module {}/{}/{}",
                    versions.len(),
                    namespace,
                    name,
                    provider
                );
                return Ok(versions);
            }
        }

        if let Some(versions) = module_versions_from_fallback_json(&json_value) {
            warn!("Using fallback module versions parsing");
            return Ok(versions);
        }

        warn!(
            "No versions available for module {}/{}/{}",
            namespace, name, provider
        );
        Err(RegistryError::NoModuleVersionsAvailable {
            module: format!("{namespace}/{name}/{provider}"),
        })
    }

    /// Get the latest version of a module
    pub async fn get_latest_module_version(
        &self,
        namespace: &str,
        name: &str,
        provider: &str,
    ) -> Result<String, RegistryError> {
        // The latest version is returned when fetching module details without a version
        let details = self
            .get_module_details(namespace, name, provider, None)
            .await?;

        if details.version.is_empty() {
            // If version is empty, try fetching versions list
            let versions = self.get_module_versions(namespace, name, provider).await?;
            versions
                .into_iter()
                .next()
                .ok_or_else(|| RegistryError::NoModuleVersionsAvailable {
                    module: format!("{namespace}/{name}/{provider}"),
                })
        } else {
            Ok(details.version)
        }
    }
}
