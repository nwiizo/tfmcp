use schemars::JsonSchema;
use serde::Deserialize;

/// Input for provider/module search
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchQueryInput {
    /// Search query string
    pub query: String,
}

/// Input for provider info lookup
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ProviderInput {
    /// Name of the provider (e.g., "aws", "google", "azurerm")
    pub provider_name: String,
    /// Provider namespace (optional, defaults to "hashicorp")
    pub namespace: Option<String>,
}

/// Input for provider documentation lookup
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ProviderDocsInput {
    /// Name of the provider
    pub provider_name: String,
    /// Service or resource name to search for
    pub service_slug: String,
    /// Provider namespace (optional)
    pub namespace: Option<String>,
    /// Type of documentation: "resources" or "data-sources"
    pub data_type: Option<String>,
}

/// Input for module details lookup
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ModuleInput {
    /// Module namespace (e.g., "terraform-aws-modules")
    pub namespace: String,
    /// Module name (e.g., "vpc")
    pub name: String,
    /// Provider name (e.g., "aws")
    pub provider: String,
    /// Specific version (optional, defaults to latest)
    pub version: Option<String>,
}

/// Input for latest module version lookup
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ModuleVersionInput {
    /// Module namespace
    pub namespace: String,
    /// Module name
    pub name: String,
    /// Provider name
    pub provider: String,
}

/// Input for policy search
#[derive(Debug, Deserialize, JsonSchema)]
pub struct PolicySearchInput {
    /// Search query for policies (e.g., "aws", "CIS", "encryption")
    pub query: String,
    /// Filter by cloud provider (e.g., "aws", "google", "azurerm")
    pub provider_filter: Option<String>,
}

/// Input for policy details lookup
#[derive(Debug, Deserialize, JsonSchema)]
pub struct PolicyDetailsInput {
    /// Policy namespace (e.g., "hashicorp")
    pub namespace: String,
    /// Policy name (e.g., "CIS-Policy-Set-for-AWS-Terraform")
    pub name: String,
}

/// Input for provider capabilities lookup
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ProviderCapabilitiesInput {
    /// Provider name (e.g., "aws", "google", "azurerm", "local")
    pub provider_name: String,
    /// Provider namespace (optional, defaults to "hashicorp")
    pub namespace: Option<String>,
}
