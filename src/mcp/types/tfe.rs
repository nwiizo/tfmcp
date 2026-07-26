//! HCP Terraform / Terraform Enterprise MCP input types.

mod common;
mod private_registry;
mod runs;
mod stacks;
mod variable_sets;
mod workspace;

pub use common::{TfeOrganizationInput, TfePageInput, TfeWorkspaceInput};
pub use private_registry::{
    TfePrivateModuleDetailsInput, TfePrivateModuleSearchInput, TfePrivateProviderDetailsInput,
    TfePrivateProviderSearchInput,
};
pub use runs::{TfeActionRunInput, TfeApplyInput, TfeCreateRunInput, TfePlanInput, TfeRunInput};
pub use stacks::{TfeStackInput, TfeStacksInput};
pub use variable_sets::{
    TfeCreateVariableInVariableSetInput, TfeCreateVariableSetInput,
    TfeDeleteVariableInVariableSetInput, TfeVariableSetWorkspacesInput, TfeVariableSetsInput,
};
pub use workspace::{
    TfeAttachPolicySetInput, TfeCreateWorkspaceInput, TfeCreateWorkspaceTagsInput,
    TfeCreateWorkspaceVariableInput, TfeUpdateWorkspaceInput, TfeUpdateWorkspaceVariableInput,
    TfeWorkspacePolicySetsInput, TfeWorkspaceRefInput, TfeWorkspaceRunsInput,
    TfeWorkspaceTagsInput, TfeWorkspaceVariablesInput,
};
