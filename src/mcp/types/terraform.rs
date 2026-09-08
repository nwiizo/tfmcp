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

/// Create a saved plan, or retrieve an existing plan without re-running Terraform.
#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct PlanInput {
    /// Existing plan ID returned by get_terraform_plan or review_terraform_plan.
    pub plan_id: Option<String>,
    /// Variable files, resolved relative to the selected project directory.
    #[serde(default)]
    pub var_files: Vec<String>,
    /// Resource addresses to replace in the saved plan.
    #[serde(default)]
    pub replace: Vec<String>,
    /// Preview externally changed objects without changing state.
    #[serde(default)]
    pub refresh_only: bool,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct PlanReferenceInput {
    /// Reuse this saved plan. Omitting it creates a new plan; pass the returned ID to subsequent tools.
    pub plan_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ApplyPlanInput {
    /// Saved plan to apply, returned by get_terraform_plan or review_terraform_plan.
    pub plan_id: Option<String>,
    /// Explicit approval of the saved plan; also requires both dangerous-operation environment gates.
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

/// Input for analyze_plan operation
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AnalyzePlanInput {
    /// Reuse this saved plan instead of generating another plan.
    pub plan_id: Option<String>,
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
