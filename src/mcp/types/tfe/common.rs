use schemars::JsonSchema;
use serde::Deserialize;

/// Pagination parameters for HCP Terraform / Terraform Enterprise list tools
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TfePageInput {
    /// Page number (default: 1)
    pub page_number: Option<u16>,
    /// Page size, clamped to 1..=100 (default: 20)
    pub page_size: Option<u16>,
}

/// Organization-scoped HCP Terraform / TFE request
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TfeOrganizationInput {
    /// Organization name
    pub organization: String,
    /// Page number (default: 1)
    pub page_number: Option<u16>,
    /// Page size, clamped to 1..=100 (default: 20)
    pub page_size: Option<u16>,
}

/// Workspace details request by ID or organization/name
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TfeWorkspaceInput {
    /// Workspace ID (e.g., ws-...)
    pub workspace_id: Option<String>,
    /// Organization name, required when workspace_id is not provided
    pub organization: Option<String>,
    /// Workspace name, required when workspace_id is not provided
    pub workspace_name: Option<String>,
}
