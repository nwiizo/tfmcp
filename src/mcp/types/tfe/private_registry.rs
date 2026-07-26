use schemars::JsonSchema;
use serde::Deserialize;

/// Private registry module search request
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TfePrivateModuleSearchInput {
    /// Organization name
    pub organization: String,
    /// Search query for module name, namespace, or provider
    pub query: Option<String>,
    /// Registry name, usually "private" (default) or "public"
    pub registry_name: Option<String>,
    /// Provider filter (e.g., "aws")
    pub provider: Option<String>,
    /// Page number (default: 1)
    pub page_number: Option<u16>,
    /// Page size, clamped to 1..=100 (default: 20)
    pub page_size: Option<u16>,
}

/// Private registry module details request
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TfePrivateModuleDetailsInput {
    /// Organization name
    pub organization: String,
    /// Registry name, usually "private" (default) or "public"
    pub registry_name: Option<String>,
    /// Module namespace. For private modules this usually matches the organization.
    pub namespace: Option<String>,
    /// Module name
    pub name: String,
    /// Module provider (e.g., "aws")
    pub provider: String,
}

/// Private registry provider search request
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TfePrivateProviderSearchInput {
    /// Organization name
    pub organization: String,
    /// Search query for provider name or namespace
    pub query: Option<String>,
    /// Registry name, usually "private" (default) or "public"
    pub registry_name: Option<String>,
    /// Page number (default: 1)
    pub page_number: Option<u16>,
    /// Page size, clamped to 1..=100 (default: 20)
    pub page_size: Option<u16>,
}

/// Private registry provider details request
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TfePrivateProviderDetailsInput {
    /// Organization name
    pub organization: String,
    /// Registry name, usually "private" (default) or "public"
    pub registry_name: Option<String>,
    /// Provider namespace. For private providers this usually matches the organization.
    pub namespace: Option<String>,
    /// Provider name
    pub name: String,
}
