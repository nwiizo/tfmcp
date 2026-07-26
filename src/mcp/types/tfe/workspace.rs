use crate::tfe::types::{
    PolicySetWorkspaceAttach, VariableAttributes, WorkspaceCreate, WorkspaceRef, WorkspaceTags,
    WorkspaceUpdate, WorkspaceVariableCreate, WorkspaceVariableUpdate,
};
use schemars::JsonSchema;
use serde::Deserialize;

/// Create workspace request
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TfeCreateWorkspaceInput {
    /// Organization name
    pub organization: String,
    /// Workspace name
    pub name: String,
    /// Optional workspace description
    pub description: Option<String>,
    /// Optional Terraform version constraint
    pub terraform_version: Option<String>,
    /// Optional execution mode, for example "remote", "local", or "agent"
    pub execution_mode: Option<String>,
    /// Optional auto-apply setting
    pub auto_apply: Option<bool>,
    /// Optional HCP Terraform project ID
    pub project_id: Option<String>,
}

/// Update workspace request by ID or organization/name
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TfeUpdateWorkspaceInput {
    /// Workspace ID (e.g., ws-...)
    pub workspace_id: Option<String>,
    /// Organization name, required when workspace_id is not provided
    pub organization: Option<String>,
    /// Workspace name, required when workspace_id is not provided
    pub workspace_name: Option<String>,
    /// Optional new workspace name
    pub new_name: Option<String>,
    /// Optional workspace description
    pub description: Option<String>,
    /// Optional Terraform version constraint
    pub terraform_version: Option<String>,
    /// Optional execution mode, for example "remote", "local", or "agent"
    pub execution_mode: Option<String>,
    /// Optional auto-apply setting
    pub auto_apply: Option<bool>,
}

/// Workspace reference by ID or organization/name
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TfeWorkspaceRefInput {
    /// Workspace ID (e.g., ws-...)
    pub workspace_id: Option<String>,
    /// Organization name, required when workspace_id is not provided
    pub organization: Option<String>,
    /// Workspace name, required when workspace_id is not provided
    pub workspace_name: Option<String>,
}

/// Workspace-scoped list request
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TfeWorkspaceRunsInput {
    /// Workspace ID (e.g., ws-...)
    pub workspace_id: String,
    /// Page number (default: 1)
    pub page_number: Option<u16>,
    /// Page size, clamped to 1..=100 (default: 20)
    pub page_size: Option<u16>,
}

/// Workspace variables request
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TfeWorkspaceVariablesInput {
    /// Workspace ID (e.g., ws-...)
    pub workspace_id: String,
}

/// Create workspace variable request
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TfeCreateWorkspaceVariableInput {
    /// Workspace ID (e.g., ws-...)
    pub workspace_id: String,
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

/// Update workspace variable request
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TfeUpdateWorkspaceVariableInput {
    /// Workspace ID (e.g., ws-...)
    pub workspace_id: String,
    /// Variable ID (e.g., var-...)
    pub variable_id: String,
    /// Optional variable key
    pub key: Option<String>,
    /// Optional variable value
    pub value: Option<String>,
    /// Optional variable category: terraform or env
    pub category: Option<String>,
    /// Optional description
    pub description: Option<String>,
    /// Whether the value is HCL
    pub hcl: Option<bool>,
    /// Whether the value is sensitive
    pub sensitive: Option<bool>,
}

/// Workspace policy set request
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TfeWorkspacePolicySetsInput {
    /// Workspace ID (e.g., ws-...)
    pub workspace_id: String,
}

/// Attach policy set to workspace request
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TfeAttachPolicySetInput {
    /// Policy set ID (e.g., polset-...)
    pub policy_set_id: String,
    /// Workspace ID (e.g., ws-...)
    pub workspace_id: String,
}

/// Workspace tag read request
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TfeWorkspaceTagsInput {
    /// Workspace ID (e.g., ws-...)
    pub workspace_id: String,
}

/// Create workspace tags request
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TfeCreateWorkspaceTagsInput {
    /// Workspace ID (e.g., ws-...)
    pub workspace_id: String,
    /// Tag names to create or attach
    pub tags: Vec<String>,
}

macro_rules! impl_from_input {
    ($source:ty => $target:ty, $input:ident { $($field:ident: $value:expr),+ $(,)? }) => {
        impl From<$source> for $target {
            fn from($input: $source) -> Self {
                Self {
                    $($field: $value),+
                }
            }
        }
    };
}

impl_from_input!(TfeCreateWorkspaceInput => WorkspaceCreate, input {
    organization: input.organization,
    name: input.name,
    description: input.description,
    terraform_version: input.terraform_version,
    execution_mode: input.execution_mode,
    auto_apply: input.auto_apply,
    project_id: input.project_id,
});

impl_from_input!(TfeUpdateWorkspaceInput => WorkspaceUpdate, input {
    workspace_id: input.workspace_id,
    organization: input.organization,
    workspace_name: input.workspace_name,
    new_name: input.new_name,
    description: input.description,
    terraform_version: input.terraform_version,
    execution_mode: input.execution_mode,
    auto_apply: input.auto_apply,
});

impl_from_input!(TfeWorkspaceRefInput => WorkspaceRef, input {
    workspace_id: input.workspace_id,
    organization: input.organization,
    workspace_name: input.workspace_name,
});

impl_from_input!(TfeCreateWorkspaceVariableInput => WorkspaceVariableCreate, input {
    workspace_id: input.workspace_id,
    variable: VariableAttributes {
        key: Some(input.key),
        value: Some(input.value),
        description: input.description,
        category: Some(input.category.unwrap_or_else(|| "terraform".to_string())),
        hcl: input.hcl,
        sensitive: input.sensitive,
    },
});

impl_from_input!(TfeUpdateWorkspaceVariableInput => WorkspaceVariableUpdate, input {
    workspace_id: input.workspace_id,
    variable_id: input.variable_id,
    variable: VariableAttributes {
        key: input.key,
        value: input.value,
        description: input.description,
        category: input.category,
        hcl: input.hcl,
        sensitive: input.sensitive,
    },
});

impl_from_input!(TfeAttachPolicySetInput => PolicySetWorkspaceAttach, input {
    policy_set_id: input.policy_set_id,
    workspace_id: input.workspace_id,
});

impl_from_input!(TfeCreateWorkspaceTagsInput => WorkspaceTags, input {
    workspace_id: input.workspace_id,
    tags: input.tags,
});
