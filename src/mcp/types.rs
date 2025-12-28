//! Input/output types for RMCP tools with automatic JSON Schema generation.

use schemars::JsonSchema;
use serde::Deserialize;

/// Input for setting Terraform directory
#[derive(Debug, Deserialize, JsonSchema)]
pub struct DirectoryInput {
    /// Path to the new Terraform project directory
    pub directory: String,
}

/// Input for apply/destroy operations
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AutoApproveInput {
    /// Whether to automatically approve the operation (default: false)
    #[serde(default)]
    pub auto_approve: bool,
}

/// Input for analyze_terraform operation
#[derive(Debug, Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct AnalyzeInput {
    /// Optional path to analyze (defaults to current project directory)
    pub path: Option<String>,
}

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
