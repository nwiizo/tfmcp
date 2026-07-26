use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum RegistryError {
    #[error("HTTP request failed: {0}")]
    HttpError(String),

    #[error("JSON parsing failed: {0}")]
    JsonError(String),

    #[error(
        "Provider '{provider}' not found in namespace '{namespace}'. Try using a different namespace or let the system auto-fallback to common namespaces (hashicorp, terraform-providers, community)."
    )]
    ProviderNotFound { provider: String, namespace: String },

    #[error(
        "Module '{module}' not found for provider '{provider}' in namespace '{namespace}'. Check the module name spelling or search for available modules."
    )]
    ModuleNotFound {
        module: String,
        provider: String,
        namespace: String,
    },

    #[error(
        "Service '{service}' not found for provider '{provider}' in namespace '{namespace}'. Check the service name spelling or browse available services first."
    )]
    #[allow(dead_code)]
    ServiceNotFound {
        service: String,
        provider: String,
        namespace: String,
    },

    #[error(
        "Documentation not found for '{doc_id}'. The documentation may have been moved or the ID may be incorrect."
    )]
    DocumentationNotFound { doc_id: String },

    #[error(
        "Invalid response format from Terraform Registry API. This may indicate a temporary service issue or API changes."
    )]
    InvalidResponse,

    #[error(
        "Rate limit exceeded. Please wait before making additional requests. The Terraform Registry has usage limits to ensure fair access."
    )]
    RateLimited,

    #[error(
        "Search returned no results for query '{query}'. Try using broader search terms or check spelling."
    )]
    NoSearchResults { query: String },

    #[error(
        "Provider '{provider}' exists but has no available versions in namespace '{namespace}'. This may indicate a deprecated or invalid provider."
    )]
    NoVersionsAvailable { provider: String, namespace: String },

    #[error(
        "Module '{module}' exists but has no available versions. This may indicate a deprecated or invalid module."
    )]
    NoModuleVersionsAvailable { module: String },
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
