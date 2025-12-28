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

// ==================== v0.1.9 New Input Types ====================

/// Input for analyze_plan operation
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AnalyzePlanInput {
    /// Include risk assessment in the analysis (default: true)
    #[serde(default = "default_true")]
    pub include_risk: bool,
}

fn default_true() -> bool {
    true
}

/// Input for analyze_state operation
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AnalyzeStateInput {
    /// Filter by resource type (e.g., "aws_instance")
    pub resource_type: Option<String>,
    /// Enable drift detection (default: false)
    #[serde(default)]
    pub detect_drift: bool,
}

/// Input for workspace operations
#[derive(Debug, Deserialize, JsonSchema)]
pub struct WorkspaceInput {
    /// Action to perform: list, show, new, select, delete
    pub action: String,
    /// Workspace name (required for new, select, delete)
    pub name: Option<String>,
}

/// Input for terraform import
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ImportInput {
    /// Resource type (e.g., "aws_instance")
    pub resource_type: String,
    /// Resource ID in the cloud provider
    pub resource_id: String,
    /// Name to use in Terraform configuration
    pub name: String,
    /// Execute the import (false = preview only)
    #[serde(default)]
    pub execute: bool,
}

/// Input for terraform fmt
#[derive(Debug, Deserialize, JsonSchema)]
pub struct FmtInput {
    /// Check only, don't modify files
    #[serde(default)]
    pub check: bool,
    /// Show diff of changes
    #[serde(default)]
    pub diff: bool,
    /// Specific file to format (optional)
    pub file: Option<String>,
}

/// Input for terraform graph
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GraphInput {
    /// Graph type: "plan" or "apply" (optional)
    pub graph_type: Option<String>,
}

/// Input for terraform output
#[derive(Debug, Deserialize, JsonSchema)]
pub struct OutputInput {
    /// Specific output name (optional, returns all if not specified)
    pub name: Option<String>,
}

/// Input for taint/untaint operations
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TaintInput {
    /// Action: "taint" or "untaint"
    pub action: String,
    /// Resource address (e.g., "aws_instance.example")
    pub address: String,
}

/// Input for terraform refresh
#[derive(Debug, Deserialize, JsonSchema)]
pub struct RefreshInput {
    /// Target specific resource (optional)
    pub target: Option<String>,
}

/// Input for terraform providers
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ProvidersInput {
    /// Include lock file information (default: false)
    #[serde(default)]
    pub include_lock: bool,
}
