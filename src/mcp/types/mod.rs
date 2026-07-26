//! Input/output types for RMCP tools with automatic JSON Schema generation.

pub mod registry;
pub mod terraform;
pub mod tfe;

pub use registry::{
    ModuleInput, ModuleVersionInput, PolicyDetailsInput, PolicySearchInput,
    ProviderCapabilitiesInput, ProviderDocsInput, ProviderInput, SearchQueryInput,
};
pub use terraform::{
    AnalyzeInput, AnalyzePlanInput, AnalyzeStateInput, AutoApproveInput, DirectoryInput, FmtInput,
    GraphInput, ImportInput, OutputInput, ProvidersInput, RefreshInput, TaintInput, WorkspaceInput,
};
pub use tfe::{
    TfeActionRunInput, TfeApplyInput, TfeAttachPolicySetInput, TfeCreateRunInput,
    TfeCreateVariableInVariableSetInput, TfeCreateVariableSetInput, TfeCreateWorkspaceInput,
    TfeCreateWorkspaceTagsInput, TfeCreateWorkspaceVariableInput,
    TfeDeleteVariableInVariableSetInput, TfeOrganizationInput, TfePageInput, TfePlanInput,
    TfePrivateModuleDetailsInput, TfePrivateModuleSearchInput, TfePrivateProviderDetailsInput,
    TfePrivateProviderSearchInput, TfeRunInput, TfeStackInput, TfeStacksInput,
    TfeUpdateWorkspaceInput, TfeUpdateWorkspaceVariableInput, TfeVariableSetWorkspacesInput,
    TfeVariableSetsInput, TfeWorkspaceInput, TfeWorkspacePolicySetsInput, TfeWorkspaceRefInput,
    TfeWorkspaceRunsInput, TfeWorkspaceTagsInput, TfeWorkspaceVariablesInput,
};
