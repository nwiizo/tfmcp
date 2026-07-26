use crate::tfe::types::{
    VariableAttributes, VariableSetCreate, VariableSetVariableCreate, VariableSetVariableDelete,
    VariableSetWorkspaces,
};
use schemars::JsonSchema;
use serde::Deserialize;

/// Organization-scoped variable set list request
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TfeVariableSetsInput {
    /// Organization name
    pub organization: String,
    /// Page number (default: 1)
    pub page_number: Option<u16>,
    /// Page size, clamped to 1..=100 (default: 20)
    pub page_size: Option<u16>,
}

/// Create variable set request
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TfeCreateVariableSetInput {
    /// Organization name
    pub organization: String,
    /// Variable set name
    pub name: String,
    /// Optional variable set description
    pub description: Option<String>,
    /// Whether the variable set applies globally
    pub global: Option<bool>,
}

/// Create variable inside variable set request
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TfeCreateVariableInVariableSetInput {
    /// Variable set ID (e.g., varset-...)
    pub variable_set_id: String,
    /// Variable key
    pub key: String,
    /// Variable value
    pub value: String,
    /// Variable category: terraform or env
    pub category: Option<String>,
    /// Optional description
    pub description: Option<String>,
    /// Whether the value is HCL
    pub hcl: Option<bool>,
    /// Whether the value is sensitive
    pub sensitive: Option<bool>,
}

/// Delete variable from variable set request
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TfeDeleteVariableInVariableSetInput {
    /// Variable set ID (e.g., varset-...)
    pub variable_set_id: String,
    /// Variable ID (e.g., var-...)
    pub variable_id: String,
}

/// Attach or detach variable set to/from workspaces request
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TfeVariableSetWorkspacesInput {
    /// Variable set ID (e.g., varset-...)
    pub variable_set_id: String,
    /// Workspace IDs (e.g., ws-...)
    pub workspace_ids: Vec<String>,
}

impl From<TfeCreateVariableSetInput> for VariableSetCreate {
    fn from(input: TfeCreateVariableSetInput) -> Self {
        Self {
            organization: input.organization,
            name: input.name,
            description: input.description,
            global: input.global,
        }
    }
}

impl From<TfeCreateVariableInVariableSetInput> for VariableSetVariableCreate {
    fn from(input: TfeCreateVariableInVariableSetInput) -> Self {
        Self {
            variable_set_id: input.variable_set_id,
            variable: VariableAttributes {
                key: Some(input.key),
                value: Some(input.value),
                description: input.description,
                category: Some(input.category.unwrap_or_else(|| "terraform".to_string())),
                hcl: input.hcl,
                sensitive: input.sensitive,
            },
        }
    }
}

impl From<TfeDeleteVariableInVariableSetInput> for VariableSetVariableDelete {
    fn from(input: TfeDeleteVariableInVariableSetInput) -> Self {
        Self {
            variable_set_id: input.variable_set_id,
            variable_id: input.variable_id,
        }
    }
}

impl From<TfeVariableSetWorkspacesInput> for VariableSetWorkspaces {
    fn from(input: TfeVariableSetWorkspacesInput) -> Self {
        Self {
            variable_set_id: input.variable_set_id,
            workspace_ids: input.workspace_ids,
        }
    }
}
