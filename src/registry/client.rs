use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use thiserror::Error;
use tracing::{debug, error, info, warn};

#[derive(Error, Debug, Clone)]
pub enum RegistryError {
    #[error("HTTP request failed: {0}")]
    HttpError(String),

    #[error("JSON parsing failed: {0}")]
    JsonError(String),

    #[error("Provider '{provider}' not found in namespace '{namespace}'. Try using a different namespace or let the system auto-fallback to common namespaces (hashicorp, terraform-providers, community).")]
    ProviderNotFound { provider: String, namespace: String },

    #[error("Service '{service}' not found for provider '{provider}' in namespace '{namespace}'. Check the service name spelling or browse available services first.")]
    #[allow(dead_code)]
    ServiceNotFound { 
        service: String, 
        provider: String, 
        namespace: String 
    },

    #[error("Documentation not found for '{doc_id}'. The documentation may have been moved or the ID may be incorrect.")]
    DocumentationNotFound { doc_id: String },

    #[error("Invalid response format from Terraform Registry API. This may indicate a temporary service issue or API changes.")]
    InvalidResponse,

    #[error("Rate limit exceeded. Please wait before making additional requests. The Terraform Registry has usage limits to ensure fair access.")]
    RateLimited,

    #[error("Search returned no results for query '{query}'. Try using broader search terms or check spelling.")]
    NoSearchResults { query: String },

    #[error("Provider '{provider}' exists but has no available versions in namespace '{namespace}'. This may indicate a deprecated or invalid provider.")]
    NoVersionsAvailable { provider: String, namespace: String },
}

impl From<reqwest::Error> for RegistryError {
    fn from(error: reqwest::Error) -> Self {
        RegistryError::HttpError(error.to_string())
    }
}

impl From<serde_json::Error> for RegistryError {
    fn from(error: serde_json::Error) -> Self {
        RegistryError::JsonError(error.to_string())
    }
}

// Flexible provider info structure that can handle multiple API versions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub name: String,
    pub namespace: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub downloads: u64,
    #[serde(default)]
    pub published_at: String,
    #[serde(default)]
    pub id: String,
    // Additional fields for API compatibility
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub tag: Option<String>,
    #[serde(default)]
    pub logo_url: Option<String>,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub tier: Option<String>,
    #[serde(default)]
    pub verified: Option<bool>,
    #[serde(default)]
    pub trusted: Option<bool>,
    // Catch unknown fields to avoid parsing failures
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl Default for ProviderInfo {
    fn default() -> Self {
        Self {
            name: String::new(),
            namespace: String::new(),
            version: String::new(),
            description: String::new(),
            downloads: 0,
            published_at: String::new(),
            id: String::new(),
            source: None,
            tag: None,
            logo_url: None,
            owner: None,
            tier: None,
            verified: None,
            trusted: None,
            extra: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocIdResult {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub slug: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub subcategory: Option<String>,
    // Catch unknown fields
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderVersions {
    #[serde(default)]
    pub versions: Vec<String>,
    // Handle alternative response formats
    #[serde(default)]
    pub data: Option<Vec<VersionInfo>>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub published_at: Option<String>,
    #[serde(default)]
    pub protocols: Option<Vec<String>>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrySearchResponse {
    #[serde(default)]
    pub providers: Vec<ProviderInfo>,
    #[serde(default)]
    pub meta: HashMap<String, Value>,
    // Handle alternative response formats
    #[serde(default)]
    pub data: Option<Vec<ProviderInfo>>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderDocsResponse {
    #[serde(default)]
    pub data: Vec<DocIdResult>,
    // Handle alternative response formats
    #[serde(default)]
    pub docs: Option<Vec<DocIdResult>>,
    #[serde(default)]
    pub documentation: Option<Vec<DocIdResult>>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

pub struct RegistryClient {
    client: Client,
    base_url: String,
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

    /// Search for providers in the Terraform Registry with improved error handling
    pub async fn search_providers(&self, query: &str) -> Result<Vec<ProviderInfo>, RegistryError> {
        let url = format!("{}/v1/providers", self.base_url);
        debug!("Searching providers with query '{}' at URL: {}", query, url);

        let response = self.client.get(&url).query(&[("q", query)]).send().await?;
        let status = response.status();
        
        debug!("Search response status: {}", status);

        if status == 429 {
            warn!("Rate limit exceeded for provider search");
            return Err(RegistryError::RateLimited);
        }

        if !status.is_success() {
            error!("HTTP error {} for search request", status);
            return Err(RegistryError::HttpError(format!("HTTP {}", status)));
        }

        let response_text = response.text().await?;
        debug!("Search response (first 1000 chars): {}", 
               &response_text.chars().take(1000).collect::<String>());

        match serde_json::from_str::<Value>(&response_text) {
            Ok(json_value) => {
                debug!("Parsed search JSON structure: {:#?}", json_value);
                
                match serde_json::from_value::<RegistrySearchResponse>(json_value.clone()) {
                    Ok(mut search_response) => {
                        // Handle alternative response formats
                        if search_response.providers.is_empty() {
                            if let Some(data) = search_response.data.take() {
                                search_response.providers = data;
                            }
                        }
                        
                        if search_response.providers.is_empty() {
                            info!("No search results found for query: {}", query);
                            return Err(RegistryError::NoSearchResults { 
                                query: query.to_string() 
                            });
                        }
                        
                        info!("Found {} providers for query: {}", search_response.providers.len(), query);
                        Ok(search_response.providers)
                    }
                    Err(e) => {
                        error!("Failed to deserialize search response: {}", e);
                        
                        // Try manual extraction
                        if let Some(providers_array) = json_value.get("providers").and_then(|v| v.as_array()) {
                            let providers = self.extract_providers_from_array(providers_array);
                            if !providers.is_empty() {
                                warn!("Using fallback provider search parsing");
                                return Ok(providers);
                            }
                        }
                        
                        Err(RegistryError::JsonError(format!("Failed to parse search response: {}", e)))
                    }
                }
            }
            Err(e) => {
                error!("Failed to parse search JSON: {}", e);
                error!("Response text was: {}", response_text);
                Err(RegistryError::JsonError(format!("Invalid JSON response: {}", e)))
            }
        }
    }

    /// Helper function to extract providers from JSON array with fallback parsing
    fn extract_providers_from_array(&self, providers_array: &[Value]) -> Vec<ProviderInfo> {
        providers_array.iter()
            .filter_map(|provider| {
                let mut provider_info = ProviderInfo::default();
                
                if let Some(name) = provider.get("name").and_then(|v| v.as_str()) {
                    provider_info.name = name.to_string();
                } else {
                    return None; // Name is required
                }
                
                if let Some(namespace) = provider.get("namespace").and_then(|v| v.as_str()) {
                    provider_info.namespace = namespace.to_string();
                }
                
                if let Some(desc) = provider.get("description").and_then(|v| v.as_str()) {
                    provider_info.description = desc.to_string();
                }
                
                if let Some(downloads) = provider.get("downloads").and_then(|v| v.as_u64()) {
                    provider_info.downloads = downloads;
                }
                
                if let Some(version) = provider.get("version").and_then(|v| v.as_str()) {
                    provider_info.version = version.to_string();
                }
                
                Some(provider_info)
            })
            .collect()
    }

    /// Get provider information by namespace and name with detailed error logging
    pub async fn get_provider_info(
        &self,
        provider_name: &str,
        namespace: &str,
    ) -> Result<ProviderInfo, RegistryError> {
        let url = format!(
            "{}/v1/providers/{}/{}",
            self.base_url, namespace, provider_name
        );

        debug!("Fetching provider info from URL: {}", url);

        let response = self.client.get(&url).send().await?;
        let status = response.status();
        
        debug!("Response status: {}", status);
        debug!("Response headers: {:?}", response.headers());

        if status == 404 {
            warn!("Provider not found: {}/{}", namespace, provider_name);
            return Err(RegistryError::ProviderNotFound {
                provider: provider_name.to_string(),
                namespace: namespace.to_string(),
            });
        }

        if status == 429 {
            warn!("Rate limit exceeded for provider info request");
            return Err(RegistryError::RateLimited);
        }

        if !status.is_success() {
            error!("HTTP error {}: {}", status, status.canonical_reason().unwrap_or("Unknown"));
            return Err(RegistryError::HttpError(format!("HTTP {}", status)));
        }

        // Get response text for detailed debugging
        let response_text = response.text().await?;
        debug!("Response body (first 1000 chars): {}", 
               &response_text.chars().take(1000).collect::<String>());

        // First try to parse as generic JSON to debug structure
        match serde_json::from_str::<Value>(&response_text) {
            Ok(json_value) => {
                debug!("Successfully parsed JSON. Structure: {:#?}", json_value);
                
                // Now try to deserialize into ProviderInfo
                match serde_json::from_value::<ProviderInfo>(json_value.clone()) {
                    Ok(provider_info) => {
                        info!("Successfully retrieved provider info for {}/{}", namespace, provider_name);
                        Ok(provider_info)
                    }
                    Err(e) => {
                        error!("Failed to deserialize ProviderInfo: {}", e);
                        error!("Parsed JSON was: {}", serde_json::to_string_pretty(&json_value).unwrap_or_else(|_| "Invalid JSON".to_string()));
                        
                        // Try to extract essential fields manually
                        let mut provider_info = ProviderInfo::default();
                        provider_info.name = json_value.get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or(provider_name)
                            .to_string();
                        provider_info.namespace = json_value.get("namespace")
                            .and_then(|v| v.as_str())
                            .unwrap_or(namespace)
                            .to_string();
                        
                        if let Some(desc) = json_value.get("description").and_then(|v| v.as_str()) {
                            provider_info.description = desc.to_string();
                        }
                        
                        if let Some(downloads) = json_value.get("downloads").and_then(|v| v.as_u64()) {
                            provider_info.downloads = downloads;
                        }
                        
                        warn!("Using fallback provider info parsing due to deserialization error");
                        Ok(provider_info)
                    }
                }
            }
            Err(e) => {
                error!("Failed to parse JSON: {}", e);
                error!("Response text was: {}", response_text);
                Err(RegistryError::JsonError(format!("Invalid JSON response: {}", e)))
            }
        }
    }

    /// Get latest version of a provider with improved error handling
    pub async fn get_latest_version(
        &self,
        provider_name: &str,
        namespace: &str,
    ) -> Result<String, RegistryError> {
        let url = format!(
            "{}/v1/providers/{}/{}/versions",
            self.base_url, namespace, provider_name
        );

        debug!("Fetching provider versions from URL: {}", url);

        let response = self.client.get(&url).send().await?;
        let status = response.status();
        
        debug!("Response status: {}", status);

        if status == 404 {
            warn!("Provider versions not found: {}/{}", namespace, provider_name);
            return Err(RegistryError::ProviderNotFound {
                provider: provider_name.to_string(),
                namespace: namespace.to_string(),
            });
        }

        if status == 429 {
            warn!("Rate limit exceeded for provider versions request");
            return Err(RegistryError::RateLimited);
        }

        if !status.is_success() {
            error!("HTTP error {}: {}", status, status.canonical_reason().unwrap_or("Unknown"));
            return Err(RegistryError::HttpError(format!("HTTP {}", status)));
        }

        let response_text = response.text().await?;
        debug!("Versions response (first 500 chars): {}", 
               &response_text.chars().take(500).collect::<String>());

        // Parse JSON and handle multiple response formats
        match serde_json::from_str::<Value>(&response_text) {
            Ok(json_value) => {
                debug!("Parsed versions JSON structure: {:#?}", json_value);
                
                // Try to deserialize into ProviderVersions
                match serde_json::from_value::<ProviderVersions>(json_value.clone()) {
                    Ok(mut versions) => {
                        // Handle alternative response formats
                        if versions.versions.is_empty() {
                            if let Some(data) = versions.data.as_ref() {
                                versions.versions = data.iter()
                                    .map(|v| v.version.clone())
                                    .filter(|v| !v.is_empty())
                                    .collect();
                            }
                        }
                        
                        if versions.versions.is_empty() {
                            warn!("No versions available for provider {}/{}", namespace, provider_name);
                            return Err(RegistryError::NoVersionsAvailable {
                                provider: provider_name.to_string(),
                                namespace: namespace.to_string(),
                            });
                        }

                        let latest_version = versions.versions.first().cloned()
                            .ok_or(RegistryError::InvalidResponse)?;
                        
                        info!("Found latest version {} for provider {}/{}", latest_version, namespace, provider_name);
                        Ok(latest_version)
                    }
                    Err(e) => {
                        error!("Failed to deserialize ProviderVersions: {}", e);
                        
                        // Try manual extraction
                        if let Some(versions_array) = json_value.get("versions").and_then(|v| v.as_array()) {
                            if let Some(first_version) = versions_array.first().and_then(|v| v.as_str()) {
                                warn!("Using fallback version parsing");
                                return Ok(first_version.to_string());
                            }
                        }
                        
                        Err(RegistryError::JsonError(format!("Failed to parse versions: {}", e)))
                    }
                }
            }
            Err(e) => {
                error!("Failed to parse versions JSON: {}", e);
                error!("Response text was: {}", response_text);
                Err(RegistryError::JsonError(format!("Invalid JSON response: {}", e)))
            }
        }
    }

    /// Search for provider documentation IDs with multiple endpoint patterns
    pub async fn search_docs(
        &self,
        provider_name: &str,
        namespace: &str,
        service_slug: &str,
        data_type: &str,
    ) -> Result<Vec<DocIdResult>, RegistryError> {
        debug!("Searching docs for provider: {}/{}, service: {}, type: {}", 
               namespace, provider_name, service_slug, data_type);

        // Try multiple URL patterns as the API endpoint may vary
        let url_patterns = vec![
            format!("{}/v1/providers/{}/{}/docs", self.base_url, namespace, provider_name),
            format!("{}/v2/providers/{}/{}/docs", self.base_url, namespace, provider_name),
            format!("{}/providers/{}/{}/docs", self.base_url, namespace, provider_name),
            format!("{}/docs/providers/{}/{}", self.base_url, namespace, provider_name),
        ];

        let query_params = vec![
            vec![("category", data_type), ("slug", service_slug)],
            vec![("type", data_type), ("slug", service_slug)],
            vec![("filter[category]", data_type), ("filter[slug]", service_slug)],
            vec![("q", service_slug), ("category", data_type)],
        ];

        for (url_idx, url) in url_patterns.iter().enumerate() {
            for (_param_idx, params) in query_params.iter().enumerate() {
                debug!("Trying URL pattern {}/{}: {} with params: {:?}", 
                       url_idx + 1, url_patterns.len(), url, params);

                let response = self.client.get(url).query(params).send().await?;
                let status = response.status();
                
                debug!("Response status: {} for URL: {}", status, url);

                if status == 429 {
                    warn!("Rate limit exceeded for docs search");
                    return Err(RegistryError::RateLimited);
                }

                if status == 404 {
                    debug!("404 for pattern {}/{}, trying next pattern", url_idx + 1, url_patterns.len());
                    continue;
                }

                if !status.is_success() {
                    warn!("HTTP error {} for docs URL: {}", status, url);
                    continue;
                }

                let response_text = response.text().await?;
                debug!("Docs response (first 500 chars): {}", 
                       &response_text.chars().take(500).collect::<String>());

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
                                    } else if let Some(documentation) = docs_response.documentation.take() {
                                        docs_response.data = documentation;
                                    }
                                }

                                if !docs_response.data.is_empty() {
                                    info!("Found {} docs for {}/{} service: {}", 
                                          docs_response.data.len(), namespace, provider_name, service_slug);
                                    return Ok(docs_response.data);
                                }
                            }
                            Err(e) => {
                                warn!("Failed to deserialize docs response: {}", e);
                                
                                // Try manual extraction from various JSON structures
                                if let Some(docs_array) = json_value.get("data").and_then(|v| v.as_array()) {
                                    let docs = self.extract_docs_from_array(docs_array);
                                    if !docs.is_empty() {
                                        info!("Extracted {} docs using fallback parsing", docs.len());
                                        return Ok(docs);
                                    }
                                }
                                
                                if let Some(docs_array) = json_value.get("docs").and_then(|v| v.as_array()) {
                                    let docs = self.extract_docs_from_array(docs_array);
                                    if !docs.is_empty() {
                                        info!("Extracted {} docs using fallback parsing (docs field)", docs.len());
                                        return Ok(docs);
                                    }
                                }

                                // Try direct array
                                if let Some(docs_array) = json_value.as_array() {
                                    let docs = self.extract_docs_from_array(docs_array);
                                    if !docs.is_empty() {
                                        info!("Extracted {} docs using fallback parsing (direct array)", docs.len());
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

        warn!("No documentation found for {}/{} service: {} after trying all patterns", 
              namespace, provider_name, service_slug);
        Ok(vec![])
    }

    /// Helper function to extract docs from JSON array with fallback parsing
    fn extract_docs_from_array(&self, docs_array: &[Value]) -> Vec<DocIdResult> {
        docs_array.iter()
            .filter_map(|doc| {
                let doc_result = DocIdResult {
                    id: doc.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    title: doc.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    description: doc.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    category: doc.get("category").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    slug: doc.get("slug").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    path: doc.get("path").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    subcategory: doc.get("subcategory").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    extra: HashMap::new(),
                };

                // Only include if we have essential fields
                if !doc_result.id.is_empty() || !doc_result.title.is_empty() {
                    Some(doc_result)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get provider documentation content by ID with multiple endpoint patterns
    pub async fn get_doc_content(&self, doc_id: &str) -> Result<String, RegistryError> {
        debug!("Fetching documentation content for ID: {}", doc_id);

        // Try multiple URL patterns for documentation content
        let url_patterns = vec![
            format!("{}/v1/docs/{}", self.base_url, doc_id),
            format!("{}/v2/docs/{}", self.base_url, doc_id),
            format!("{}/docs/{}", self.base_url, doc_id),
            format!("{}/documentation/{}", self.base_url, doc_id),
        ];

        for (idx, url) in url_patterns.iter().enumerate() {
            debug!("Trying documentation URL pattern {}/{}: {}", idx + 1, url_patterns.len(), url);

            let response = self.client.get(url).send().await?;
            let status = response.status();
            
            debug!("Response status: {} for docs URL: {}", status, url);

            if status == 429 {
                warn!("Rate limit exceeded for documentation content");
                return Err(RegistryError::RateLimited);
            }

            if status == 404 {
                debug!("404 for docs pattern {}/{}, trying next pattern", idx + 1, url_patterns.len());
                continue;
            }

            if !status.is_success() {
                warn!("HTTP error {} for docs content URL: {}", status, url);
                continue;
            }

            let content = response.text().await?;
            debug!("Retrieved documentation content ({} chars) for ID: {}", content.len(), doc_id);
            
            if !content.trim().is_empty() {
                info!("Successfully retrieved documentation content for ID: {}", doc_id);
                return Ok(content);
            }
        }

        error!("Documentation not found for ID: {} after trying all patterns", doc_id);
        Err(RegistryError::DocumentationNotFound {
            doc_id: doc_id.to_string(),
        })
    }

}
