use crate::tfe::types::RunCreate;
use schemars::JsonSchema;
use serde::Deserialize;

/// Run details request
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TfeRunInput {
    /// Run ID (e.g., run-...)
    pub run_id: String,
}

/// Create run request
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TfeCreateRunInput {
    /// Workspace ID (e.g., ws-...)
    pub workspace_id: String,
    /// Optional run message
    pub message: Option<String>,
    /// Optional run operation, for example "plan_only", "plan_and_apply", "refresh_only", or "destroy"
    pub operation: Option<String>,
    /// Whether this run should destroy resources
    pub is_destroy: Option<bool>,
    /// Whether to refresh state during this run
    pub refresh: Option<bool>,
    /// Whether this should be a speculative plan-only run
    pub plan_only: Option<bool>,
    /// Optional auto-apply setting for the run
    pub auto_apply: Option<bool>,
}

/// Run action request
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TfeActionRunInput {
    /// Run ID (e.g., run-...)
    pub run_id: String,
    /// Action: apply, discard, cancel, force-cancel, or force-execute
    pub action: String,
}

/// Plan request
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TfePlanInput {
    /// Plan ID (e.g., plan-...)
    pub plan_id: String,
}

/// Apply request
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TfeApplyInput {
    /// Apply ID (e.g., apply-...)
    pub apply_id: String,
}

impl From<TfeCreateRunInput> for RunCreate {
    fn from(input: TfeCreateRunInput) -> Self {
        Self {
            workspace_id: input.workspace_id,
            message: input.message,
            operation: input.operation,
            is_destroy: input.is_destroy,
            refresh: input.refresh,
            plan_only: input.plan_only,
            auto_apply: input.auto_apply,
        }
    }
}
