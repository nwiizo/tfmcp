//! RMCP-based MCP server implementation for tfmcp.

mod analysis;
mod protocol;

use analysis::AnalysisToolCall;

use crate::core::tfmcp::TfMcp;
use crate::mcp::deployment::DeploymentControls;
pub use crate::mcp::tool_filter::ToolFilter;
use crate::mcp::types::*;
use crate::registry::fallback::RegistryClientWithFallback;
use crate::registry::policy::PolicyClient;
use crate::registry::provider::ProviderResolver;
use crate::shared::logging;
use crate::shared::metrics;
use crate::shared::security::{SecurityManager, SecurityPolicy};
use crate::tfe::client::{PageParams, TfeClient};
use rmcp::{
    ErrorData as McpError,
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{CacheScope, CallToolResult, ContentBlock, ListToolsResult, Tool},
    service::{RequestContext, RoleServer, ServiceExt},
    tool, tool_router,
};
use std::fmt::Display;
use std::future::Future;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Mutex, RwLock};

const MCP_CACHE_TTL_MS: u64 = 300_000;

fn json_success(value: &impl serde::Serialize) -> Result<CallToolResult, McpError> {
    let structured = serde_json::to_value(value)
        .map_err(|e| McpError::internal_error(format!("JSON serialization failed: {e}"), None))?;
    let text = serde_json::to_string_pretty(&structured)
        .map_err(|e| McpError::internal_error(format!("JSON serialization failed: {e}"), None))?;
    let mut result = CallToolResult::structured(structured);
    result.content = vec![ContentBlock::text(text)];
    Ok(result)
}

fn text_error(prefix: &str, error: impl Display) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(format!("{prefix}: {error}"))])
}

type R = Result<CallToolResult, McpError>;
type C = RequestContext<RoleServer>;
type P<T> = Parameters<T>;
type PWorkspacePolicySets = P<TfeWorkspacePolicySetsInput>;
type PAttachPolicySet = P<TfeAttachPolicySetInput>;
type PVariableSets = P<TfeVariableSetsInput>;
type PCreateVariableSet = P<TfeCreateVariableSetInput>;
type PCreateVariableInSet = P<TfeCreateVariableInVariableSetInput>;
type PDeleteVariableInSet = P<TfeDeleteVariableInVariableSetInput>;
type PVariableSetWorkspaces = P<TfeVariableSetWorkspacesInput>;
type PWorkspaceTags = P<TfeWorkspaceTagsInput>;
type PCreateWorkspaceTags = P<TfeCreateWorkspaceTagsInput>;
type PStacks = P<TfeStacksInput>;
type PStack = P<TfeStackInput>;

async fn json_result<T, E, Fut>(future: Fut, key: &'static str) -> Result<serde_json::Value, String>
where
    T: serde::Serialize,
    E: Display,
    Fut: Future<Output = Result<T, E>>,
{
    future
        .await
        .map(|value| serde_json::json!({ key: value }))
        .map_err(|e| e.to_string())
}

fn page_params(number: Option<u16>, size: Option<u16>) -> PageParams {
    PageParams::new(number, size)
}

fn default_audit_manager() -> SecurityManager {
    SecurityManager::new().unwrap_or_else(|e| {
        logging::error(&format!("Failed to initialize audit manager: {e}"));
        SecurityManager {
            policy: SecurityPolicy::default(),
            audit_log: None,
        }
    })
}

fn default_deployment_controls() -> DeploymentControls {
    DeploymentControls::from_env().unwrap_or_else(|e| {
        logging::error(&format!("Failed to initialize deployment controls: {e}"));
        DeploymentControls::default()
    })
}

macro_rules! tfe_call_value {
    ($server:expr, $call:expr) => {
        match $call {
            TfeToolCall::TokenPermissions => $server.token_permissions_value().await,
            TfeToolCall::ListOrganizations(input) => {
                json_result(
                    $server
                        .tfe_client
                        .list_organizations(page_params(input.page_number, input.page_size)),
                    "organizations",
                )
                .await
            }
            TfeToolCall::ListProjects(input) => {
                json_result(
                    $server.tfe_client.list_projects(
                        &input.organization,
                        page_params(input.page_number, input.page_size),
                    ),
                    "projects",
                )
                .await
            }
            TfeToolCall::ListWorkspaces(input) => {
                json_result(
                    $server.tfe_client.list_workspaces(
                        &input.organization,
                        page_params(input.page_number, input.page_size),
                    ),
                    "workspaces",
                )
                .await
            }
            TfeToolCall::WorkspaceDetails(input) => $server.workspace_details_value(input).await,
            TfeToolCall::ListRuns(input) => {
                json_result(
                    $server.tfe_client.list_runs(
                        &input.workspace_id,
                        page_params(input.page_number, input.page_size),
                    ),
                    "runs",
                )
                .await
            }
            TfeToolCall::RunDetails(input) => $server.run_details_value(input).await,
            TfeToolCall::PlanDetails(input) => $server.plan_details_value(input).await,
            TfeToolCall::PlanLogs(input) => $server.plan_logs_value(input).await,
            TfeToolCall::PlanJsonOutput(input) => $server.plan_json_output_value(input).await,
            TfeToolCall::ApplyDetails(input) => $server.apply_details_value(input).await,
            TfeToolCall::ApplyLogs(input) => $server.apply_logs_value(input).await,
            TfeToolCall::SearchPrivateModules(input) => {
                json_result(
                    $server.tfe_client.search_registry_items(
                        &input.organization,
                        "registry-modules",
                        input.query.as_deref(),
                        input.registry_name.as_deref().or(Some("private")),
                        input.provider.as_deref(),
                        page_params(input.page_number, input.page_size),
                    ),
                    "modules",
                )
                .await
            }
            TfeToolCall::PrivateModuleDetails(input) => {
                let registry_name = input.registry_name.as_deref().unwrap_or("private");
                let namespace = input.namespace.as_deref().unwrap_or(&input.organization);
                json_result(
                    $server.tfe_client.get_registry_item(
                        &input.organization,
                        "registry-modules",
                        registry_name,
                        namespace,
                        &input.name,
                        Some(&input.provider),
                    ),
                    "module",
                )
                .await
            }
            TfeToolCall::SearchPrivateProviders(input) => {
                json_result(
                    $server.tfe_client.search_registry_items(
                        &input.organization,
                        "registry-providers",
                        input.query.as_deref(),
                        input.registry_name.as_deref().or(Some("private")),
                        None,
                        page_params(input.page_number, input.page_size),
                    ),
                    "providers",
                )
                .await
            }
            TfeToolCall::PrivateProviderDetails(input) => {
                let registry_name = input.registry_name.as_deref().unwrap_or("private");
                let namespace = input.namespace.as_deref().unwrap_or(&input.organization);
                json_result(
                    $server.tfe_client.get_registry_item(
                        &input.organization,
                        "registry-providers",
                        registry_name,
                        namespace,
                        &input.name,
                        None,
                    ),
                    "provider",
                )
                .await
            }
            TfeToolCall::CreateWorkspace(input) => {
                json_result(
                    $server.tfe_client.create_workspace(input.into()),
                    "workspace",
                )
                .await
            }
            TfeToolCall::UpdateWorkspace(input) => {
                json_result(
                    $server.tfe_client.update_workspace(input.into()),
                    "workspace",
                )
                .await
            }
            TfeToolCall::DeleteWorkspaceSafely(input) => {
                json_result(
                    $server.tfe_client.safe_delete_workspace(input.into()),
                    "workspace",
                )
                .await
            }
            TfeToolCall::CreateRun(input) => {
                json_result($server.tfe_client.create_run(input.into()), "run").await
            }
            TfeToolCall::ActionRun(input) => {
                json_result(
                    $server.tfe_client.action_run(&input.run_id, &input.action),
                    "run_action",
                )
                .await
            }
            TfeToolCall::ListWorkspaceVariables(input) => {
                json_result(
                    $server
                        .tfe_client
                        .list_workspace_variables(&input.workspace_id),
                    "variables",
                )
                .await
            }
            TfeToolCall::CreateWorkspaceVariable(input) => {
                json_result(
                    $server.tfe_client.create_workspace_variable(input.into()),
                    "variable",
                )
                .await
            }
            TfeToolCall::UpdateWorkspaceVariable(input) => {
                json_result(
                    $server.tfe_client.update_workspace_variable(input.into()),
                    "variable",
                )
                .await
            }
            TfeToolCall::WorkspacePolicySets(input) => {
                json_result(
                    $server
                        .tfe_client
                        .get_workspace_policy_sets(&input.workspace_id),
                    "policy_sets",
                )
                .await
            }
            TfeToolCall::AttachPolicySetToWorkspace(input) => {
                json_result(
                    $server
                        .tfe_client
                        .attach_policy_set_to_workspace(input.into()),
                    "policy_set_attachment",
                )
                .await
            }
            TfeToolCall::ListVariableSets(input) => {
                json_result(
                    $server.tfe_client.list_variable_sets(
                        &input.organization,
                        page_params(input.page_number, input.page_size),
                    ),
                    "variable_sets",
                )
                .await
            }
            TfeToolCall::CreateVariableSet(input) => {
                json_result(
                    $server.tfe_client.create_variable_set(input.into()),
                    "variable_set",
                )
                .await
            }
            TfeToolCall::CreateVariableInVariableSet(input) => {
                json_result(
                    $server
                        .tfe_client
                        .create_variable_in_variable_set(input.into()),
                    "variable",
                )
                .await
            }
            TfeToolCall::DeleteVariableInVariableSet(input) => {
                json_result(
                    $server
                        .tfe_client
                        .delete_variable_in_variable_set(input.into()),
                    "variable",
                )
                .await
            }
            TfeToolCall::AttachVariableSetToWorkspaces(input) => {
                json_result(
                    $server
                        .tfe_client
                        .attach_variable_set_to_workspaces(input.into()),
                    "variable_set_attachment",
                )
                .await
            }
            TfeToolCall::DetachVariableSetFromWorkspaces(input) => {
                json_result(
                    $server
                        .tfe_client
                        .detach_variable_set_from_workspaces(input.into()),
                    "variable_set_detachment",
                )
                .await
            }
            TfeToolCall::ReadWorkspaceTags(input) => {
                json_result(
                    $server.tfe_client.read_workspace_tags(&input.workspace_id),
                    "tags",
                )
                .await
            }
            TfeToolCall::CreateWorkspaceTags(input) => {
                json_result(
                    $server.tfe_client.create_workspace_tags(input.into()),
                    "tags",
                )
                .await
            }
            TfeToolCall::ListStacks(input) => {
                json_result(
                    $server.tfe_client.list_stacks(
                        &input.organization,
                        page_params(input.page_number, input.page_size),
                    ),
                    "stacks",
                )
                .await
            }
            TfeToolCall::StackDetails(input) => {
                json_result($server.tfe_client.get_stack(&input.stack_id), "stack").await
            }
        }
    };
}

enum TfmcpToolCall {
    ListResources,
    Plan,
    Apply(bool),
    Destroy(bool),
    Init,
    Validate,
    ValidateDetailed,
    State,
    InspectProject,
    DetectEntrypoints,
    DependencyGraph,
    SuggestRefactoring,
    AnalyzePlan(bool),
    ReviewPlan,
    SummarizePlanForPr,
    AnalyzeState {
        resource_type: Option<String>,
        detect_drift: bool,
    },
    Workspace {
        action: String,
        name: Option<String>,
    },
    Import(ImportInput),
    Fmt(FmtInput),
    Graph(Option<String>),
    Output(Option<String>),
    Taint(TaintInput),
    Refresh(Option<String>),
    Providers(bool),
    CheckProviderLockfile,
    QualityChecks,
    InspectStateSafety,
    DetectDriftCandidates,
    PrepareTerraformChange,
}

impl TfmcpToolCall {
    fn tool_name(&self) -> &'static str {
        match self {
            Self::ListResources => "list_terraform_resources",
            Self::Plan => "get_terraform_plan",
            Self::Apply(_) => "apply_terraform",
            Self::Destroy(_) => "destroy_terraform",
            Self::Init => "init_terraform",
            Self::Validate => "validate_terraform",
            Self::ValidateDetailed => "validate_terraform_detailed",
            Self::State => "get_terraform_state",
            Self::InspectProject => "inspect_terraform_project",
            Self::DetectEntrypoints => "detect_terraform_entrypoints",
            Self::DependencyGraph => "get_resource_dependency_graph",
            Self::SuggestRefactoring => "suggest_module_refactoring",
            Self::AnalyzePlan(_) => "analyze_plan",
            Self::ReviewPlan => "review_terraform_plan",
            Self::SummarizePlanForPr => "summarize_plan_for_pr",
            Self::AnalyzeState { .. } => "analyze_state",
            Self::Workspace { .. } => "terraform_workspace",
            Self::Import(_) => "terraform_import",
            Self::Fmt(_) => "terraform_fmt",
            Self::Graph(_) => "terraform_graph",
            Self::Output(_) => "terraform_output",
            Self::Taint(_) => "terraform_taint",
            Self::Refresh(_) => "terraform_refresh",
            Self::Providers(_) => "terraform_providers",
            Self::CheckProviderLockfile => "check_provider_lockfile",
            Self::QualityChecks => "run_terraform_quality_checks",
            Self::InspectStateSafety => "inspect_state_safety",
            Self::DetectDriftCandidates => "detect_drift_candidates",
            Self::PrepareTerraformChange => "prepare_terraform_change",
        }
    }

    fn error_prefix(&self) -> &'static str {
        match self {
            Self::ListResources => "Failed to list resources",
            Self::Plan => "Failed to get plan",
            Self::Apply(_) => "Failed to apply",
            Self::Destroy(_) => "Failed to destroy",
            Self::Init => "Failed to init",
            Self::Validate => "Validation failed",
            Self::ValidateDetailed => "Detailed validation failed",
            Self::State => "Failed to get state",
            Self::InspectProject => "Failed to inspect Terraform project",
            Self::DetectEntrypoints => "Failed to detect Terraform entrypoints",
            Self::DependencyGraph => "Failed to get dependency graph",
            Self::SuggestRefactoring => "Failed to get refactoring suggestions",
            Self::AnalyzePlan(_) => "Plan analysis failed",
            Self::ReviewPlan => "Plan review failed",
            Self::SummarizePlanForPr => "Plan PR summary failed",
            Self::AnalyzeState { .. } => "State analysis failed",
            Self::Workspace { .. } => "Workspace operation failed",
            Self::Import(_) => "Import failed",
            Self::Fmt(_) => "Format failed",
            Self::Graph(_) => "Graph generation failed",
            Self::Output(_) => "Output retrieval failed",
            Self::Taint(_) => "Taint operation failed",
            Self::Refresh(_) => "Refresh failed",
            Self::Providers(_) => "Provider info failed",
            Self::CheckProviderLockfile => "Provider lockfile check failed",
            Self::QualityChecks => "Terraform quality checks failed",
            Self::InspectStateSafety => "State safety inspection failed",
            Self::DetectDriftCandidates => "Drift candidate detection failed",
            Self::PrepareTerraformChange => "Terraform change preparation failed",
        }
    }
}

enum RegistryToolCall {
    SearchProviders(String),
    ProviderInfo(ProviderInput),
    ProviderDocs(ProviderDocsInput),
    SearchModules(String),
    ModuleDetails(ModuleInput),
    SearchPolicies(PolicySearchInput),
    PolicyDetails(PolicyDetailsInput),
}

enum TfeToolCall {
    TokenPermissions,
    ListOrganizations(TfePageInput),
    ListProjects(TfeOrganizationInput),
    ListWorkspaces(TfeOrganizationInput),
    WorkspaceDetails(TfeWorkspaceInput),
    ListRuns(TfeWorkspaceRunsInput),
    RunDetails(TfeRunInput),
    PlanDetails(TfePlanInput),
    PlanLogs(TfePlanInput),
    PlanJsonOutput(TfePlanInput),
    ApplyDetails(TfeApplyInput),
    ApplyLogs(TfeApplyInput),
    SearchPrivateModules(TfePrivateModuleSearchInput),
    PrivateModuleDetails(TfePrivateModuleDetailsInput),
    SearchPrivateProviders(TfePrivateProviderSearchInput),
    PrivateProviderDetails(TfePrivateProviderDetailsInput),
    CreateWorkspace(TfeCreateWorkspaceInput),
    UpdateWorkspace(TfeUpdateWorkspaceInput),
    DeleteWorkspaceSafely(TfeWorkspaceRefInput),
    CreateRun(TfeCreateRunInput),
    ActionRun(TfeActionRunInput),
    ListWorkspaceVariables(TfeWorkspaceVariablesInput),
    CreateWorkspaceVariable(TfeCreateWorkspaceVariableInput),
    UpdateWorkspaceVariable(TfeUpdateWorkspaceVariableInput),
    WorkspacePolicySets(TfeWorkspacePolicySetsInput),
    AttachPolicySetToWorkspace(TfeAttachPolicySetInput),
    ListVariableSets(TfeVariableSetsInput),
    CreateVariableSet(TfeCreateVariableSetInput),
    CreateVariableInVariableSet(TfeCreateVariableInVariableSetInput),
    DeleteVariableInVariableSet(TfeDeleteVariableInVariableSetInput),
    AttachVariableSetToWorkspaces(TfeVariableSetWorkspacesInput),
    DetachVariableSetFromWorkspaces(TfeVariableSetWorkspacesInput),
    ReadWorkspaceTags(TfeWorkspaceTagsInput),
    CreateWorkspaceTags(TfeCreateWorkspaceTagsInput),
    ListStacks(TfeStacksInput),
    StackDetails(TfeStackInput),
}

impl TfeToolCall {
    fn metadata(&self) -> (&'static str, &'static str) {
        match self {
            Self::TokenPermissions => ("get_token_permissions", "Token permission lookup failed"),
            Self::ListOrganizations(_) => ("list_terraform_orgs", "Organization listing failed"),
            Self::ListProjects(_) => ("list_terraform_projects", "Project listing failed"),
            Self::ListWorkspaces(_) => ("list_workspaces", "Workspace listing failed"),
            Self::WorkspaceDetails(_) => ("get_workspace_details", "Workspace lookup failed"),
            Self::ListRuns(_) => ("list_runs", "Run listing failed"),
            Self::RunDetails(_) => ("get_run_details", "Run lookup failed"),
            Self::PlanDetails(_) => ("get_plan_details", "Plan lookup failed"),
            Self::PlanLogs(_) => ("get_plan_logs", "Plan log lookup failed"),
            Self::PlanJsonOutput(_) => ("get_plan_json_output", "Plan JSON output lookup failed"),
            Self::ApplyDetails(_) => ("get_apply_details", "Apply lookup failed"),
            Self::ApplyLogs(_) => ("get_apply_logs", "Apply log lookup failed"),
            Self::SearchPrivateModules(_) => {
                ("search_private_modules", "Private module search failed")
            }
            Self::PrivateModuleDetails(_) => {
                ("get_private_module_details", "Private module lookup failed")
            }
            Self::SearchPrivateProviders(_) => {
                ("search_private_providers", "Private provider search failed")
            }
            Self::PrivateProviderDetails(_) => (
                "get_private_provider_details",
                "Private provider lookup failed",
            ),
            Self::CreateWorkspace(_) => ("create_workspace", "Workspace creation failed"),
            Self::UpdateWorkspace(_) => ("update_workspace", "Workspace update failed"),
            Self::DeleteWorkspaceSafely(_) => {
                ("delete_workspace_safely", "Safe workspace deletion failed")
            }
            Self::CreateRun(_) => ("create_run", "Run creation failed"),
            Self::ActionRun(_) => ("action_run", "Run action failed"),
            Self::ListWorkspaceVariables(_) => (
                "list_workspace_variables",
                "Workspace variable listing failed",
            ),
            Self::CreateWorkspaceVariable(_) => (
                "create_workspace_variable",
                "Workspace variable creation failed",
            ),
            Self::UpdateWorkspaceVariable(_) => (
                "update_workspace_variable",
                "Workspace variable update failed",
            ),
            Self::WorkspacePolicySets(_) => (
                "get_workspace_policy_sets",
                "Workspace policy set listing failed",
            ),
            Self::AttachPolicySetToWorkspace(_) => (
                "attach_policy_set_to_workspace",
                "Policy set workspace attachment failed",
            ),
            Self::ListVariableSets(_) => ("list_variable_sets", "Variable set listing failed"),
            Self::CreateVariableSet(_) => ("create_variable_set", "Variable set creation failed"),
            Self::CreateVariableInVariableSet(_) => (
                "create_variable_in_variable_set",
                "Variable set variable creation failed",
            ),
            Self::DeleteVariableInVariableSet(_) => (
                "delete_variable_in_variable_set",
                "Variable set variable deletion failed",
            ),
            Self::AttachVariableSetToWorkspaces(_) => (
                "attach_variable_set_to_workspaces",
                "Variable set workspace attachment failed",
            ),
            Self::DetachVariableSetFromWorkspaces(_) => (
                "detach_variable_set_from_workspaces",
                "Variable set workspace detachment failed",
            ),
            Self::ReadWorkspaceTags(_) => ("read_workspace_tags", "Workspace tag lookup failed"),
            Self::CreateWorkspaceTags(_) => {
                ("create_workspace_tags", "Workspace tag creation failed")
            }
            Self::ListStacks(_) => ("list_stacks", "Stack listing failed"),
            Self::StackDetails(_) => ("get_stack_details", "Stack lookup failed"),
        }
    }

    fn requires_audit(&self) -> bool {
        matches!(
            self,
            Self::CreateWorkspace(_)
                | Self::UpdateWorkspace(_)
                | Self::DeleteWorkspaceSafely(_)
                | Self::CreateRun(_)
                | Self::ActionRun(_)
                | Self::CreateWorkspaceVariable(_)
                | Self::UpdateWorkspaceVariable(_)
                | Self::AttachPolicySetToWorkspace(_)
                | Self::CreateVariableSet(_)
                | Self::CreateVariableInVariableSet(_)
                | Self::DeleteVariableInVariableSet(_)
                | Self::AttachVariableSetToWorkspaces(_)
                | Self::DetachVariableSetFromWorkspaces(_)
                | Self::CreateWorkspaceTags(_)
        )
    }

    fn organization(&self) -> Option<&str> {
        match self {
            Self::ListProjects(input) => Some(&input.organization),
            Self::ListWorkspaces(input) => Some(&input.organization),
            Self::WorkspaceDetails(input) if input.workspace_id.is_none() => {
                input.organization.as_deref()
            }
            Self::SearchPrivateModules(input) => Some(&input.organization),
            Self::PrivateModuleDetails(input) => Some(&input.organization),
            Self::SearchPrivateProviders(input) => Some(&input.organization),
            Self::PrivateProviderDetails(input) => Some(&input.organization),
            Self::CreateWorkspace(input) => Some(&input.organization),
            Self::UpdateWorkspace(input) if input.workspace_id.is_none() => {
                input.organization.as_deref()
            }
            Self::DeleteWorkspaceSafely(input) if input.workspace_id.is_none() => {
                input.organization.as_deref()
            }
            Self::ListVariableSets(input) => Some(&input.organization),
            Self::CreateVariableSet(input) => Some(&input.organization),
            Self::ListStacks(input) => Some(&input.organization),
            _ => None,
        }
    }
}

impl RegistryToolCall {
    fn tool_name(&self) -> &'static str {
        match self {
            Self::SearchProviders(_) => "search_terraform_providers",
            Self::ProviderInfo(_) => "get_provider_info",
            Self::ProviderDocs(_) => "get_provider_docs",
            Self::SearchModules(_) => "search_terraform_modules",
            Self::ModuleDetails(_) => "get_module_details",
            Self::SearchPolicies(_) => "search_policies",
            Self::PolicyDetails(_) => "get_policy_details",
        }
    }

    fn error_prefix(&self) -> &'static str {
        match self {
            Self::SearchProviders(_) => "Provider search failed",
            Self::ProviderInfo(_) => "Failed to get provider info",
            Self::ProviderDocs(_) => "Failed to get provider docs",
            Self::SearchModules(_) => "Module search failed",
            Self::ModuleDetails(_) => "Failed to get module details",
            Self::SearchPolicies(_) => "Policy search failed",
            Self::PolicyDetails(_) => "Policy details failed",
        }
    }
}

/// RMCP-based MCP server for Terraform operations.
#[derive(Clone)]
pub struct TfMcpServer {
    tfmcp: Arc<RwLock<TfMcp>>,
    terraform_operation_lock: Arc<Mutex<()>>,
    registry_client: Arc<RegistryClientWithFallback>,
    provider_resolver: Arc<ProviderResolver>,
    policy_client: Arc<PolicyClient>,
    tfe_client: Arc<TfeClient>,
    audit_manager: Arc<SecurityManager>,
    deployment_controls: Arc<DeploymentControls>,
    tool_filter: ToolFilter,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl TfMcpServer {
    /// Create a new TfMcpServer instance.
    pub fn new(tfmcp: TfMcp, tool_filter: ToolFilter) -> Self {
        Self::new_with_tfe_client(tfmcp, tool_filter, TfeClient::from_env())
    }

    /// Create a new TfMcpServer instance with an explicit TFE client.
    pub fn new_with_tfe_client(
        tfmcp: TfMcp,
        tool_filter: ToolFilter,
        tfe_client: TfeClient,
    ) -> Self {
        Self::new_with_tfe_client_and_audit_manager(
            tfmcp,
            tool_filter,
            tfe_client,
            default_audit_manager(),
        )
    }

    /// Create a new TfMcpServer instance with explicit TFE and audit dependencies.
    pub fn new_with_tfe_client_and_audit_manager(
        tfmcp: TfMcp,
        tool_filter: ToolFilter,
        tfe_client: TfeClient,
        audit_manager: SecurityManager,
    ) -> Self {
        Self {
            tfmcp: Arc::new(RwLock::new(tfmcp)),
            terraform_operation_lock: Arc::new(Mutex::new(())),
            registry_client: Arc::new(RegistryClientWithFallback::new()),
            provider_resolver: Arc::new(ProviderResolver::new()),
            policy_client: Arc::new(PolicyClient::new()),
            tfe_client: Arc::new(tfe_client),
            audit_manager: Arc::new(audit_manager),
            deployment_controls: Arc::new(default_deployment_controls()),
            tool_filter,
            tool_router: Self::tool_router(),
        }
    }

    /// Return a server clone with explicit remote deployment controls.
    pub fn with_deployment_controls(mut self, deployment_controls: DeploymentControls) -> Self {
        self.deployment_controls = Arc::new(deployment_controls);
        self
    }

    /// Serve the MCP server over stdio with optional tool filtering.
    pub async fn serve_stdio(tfmcp: TfMcp, tool_filter: ToolFilter) -> anyhow::Result<()> {
        use tokio::io::{stdin, stdout};

        let server = Self::new(tfmcp, tool_filter);
        let transport = (stdin(), stdout());

        logging::info("Starting tfmcp MCP server via stdio...");
        let service = server.serve(transport).await?;

        // Wait for the server to finish (keep it alive)
        service.waiting().await?;

        Ok(())
    }

    fn filtered_tools(&self) -> Vec<Tool> {
        self.tool_router
            .list_all()
            .into_iter()
            .filter(|tool| self.tool_filter.is_enabled(tool.name.as_ref()))
            .collect()
    }

    fn list_tools_result(&self) -> Result<ListToolsResult, McpError> {
        Ok(ListToolsResult::with_all_items(self.filtered_tools())
            .with_ttl_ms(MCP_CACHE_TTL_MS)
            .with_cache_scope(CacheScope::Public))
    }

    async fn run_tfmcp_call(&self, call: TfmcpToolCall) -> Result<CallToolResult, McpError> {
        let tool_name = call.tool_name();
        let error_prefix = call.error_prefix();
        self.run_measured_tool_call(tool_name, error_prefix, self.tfmcp_call_value(call))
            .await
    }

    async fn tfmcp_call_value(&self, call: TfmcpToolCall) -> Result<serde_json::Value, String> {
        // Terraform CLI commands share a working directory and state. Serialize
        // them even when MCP requests arrive concurrently over HTTP.
        let _operation_guard = self.terraform_operation_lock.lock().await;
        let tfmcp = self.tfmcp.read().await;

        match call {
            TfmcpToolCall::ListResources => tfmcp
                .list_resources()
                .await
                .map(|resources| serde_json::json!({ "resources": resources })),
            TfmcpToolCall::Plan => tfmcp
                .get_terraform_plan()
                .await
                .map(|plan| serde_json::json!({ "plan": plan })),
            TfmcpToolCall::Apply(auto_approve) => tfmcp
                .apply_terraform(auto_approve)
                .await
                .map(|output| serde_json::json!({ "output": output })),
            TfmcpToolCall::Destroy(auto_approve) => tfmcp
                .destroy_terraform(auto_approve)
                .await
                .map(|output| serde_json::json!({ "output": output })),
            TfmcpToolCall::Init => tfmcp
                .init_terraform()
                .await
                .map(|output| serde_json::json!({ "output": output })),
            TfmcpToolCall::Validate => tfmcp.validate_configuration().await.map(|message| {
                serde_json::json!({
                    "valid": !message.contains("Error:"),
                    "message": message
                })
            }),
            TfmcpToolCall::ValidateDetailed => tfmcp
                .validate_configuration_detailed()
                .await
                .map(|result| serde_json::json!(result)),
            TfmcpToolCall::State => tfmcp
                .get_state()
                .await
                .map(|state| serde_json::json!({ "state": state })),
            TfmcpToolCall::InspectProject => tfmcp
                .inspect_project()
                .await
                .map(|inspection| serde_json::json!(inspection)),
            TfmcpToolCall::DetectEntrypoints => tfmcp
                .detect_entrypoints()
                .await
                .map(|entrypoints| serde_json::json!({ "entrypoints": entrypoints })),
            TfmcpToolCall::DependencyGraph => tfmcp
                .get_dependency_graph()
                .await
                .map(|graph| serde_json::json!(graph)),
            TfmcpToolCall::SuggestRefactoring => tfmcp
                .suggest_refactoring()
                .await
                .map(|suggestions| serde_json::json!({ "suggestions": suggestions })),
            TfmcpToolCall::AnalyzePlan(include_risk) => tfmcp
                .analyze_plan(include_risk)
                .await
                .map(|analysis| serde_json::json!(analysis)),
            TfmcpToolCall::ReviewPlan => tfmcp
                .review_plan()
                .await
                .map(|review| serde_json::json!(review)),
            TfmcpToolCall::SummarizePlanForPr => tfmcp
                .summarize_plan_for_pr()
                .await
                .map(|summary| serde_json::json!(summary)),
            TfmcpToolCall::AnalyzeState {
                resource_type,
                detect_drift,
            } => tfmcp
                .analyze_state(resource_type.as_deref(), detect_drift)
                .await
                .map(|analysis| serde_json::json!(analysis)),
            TfmcpToolCall::Workspace { action, name } => tfmcp
                .workspace(&action, name.as_deref())
                .await
                .map(|result| serde_json::json!(result)),
            TfmcpToolCall::Import(input) => tfmcp
                .import_resource(
                    &input.resource_type,
                    &input.resource_id,
                    &input.name,
                    input.execute,
                )
                .await
                .map(|result| serde_json::json!(result)),
            TfmcpToolCall::Fmt(input) => tfmcp
                .fmt(input.check, input.diff, input.file.as_deref())
                .await
                .map(|result| serde_json::json!(result)),
            TfmcpToolCall::Graph(graph_type) => tfmcp
                .graph(graph_type.as_deref())
                .await
                .map(|graph| serde_json::json!(graph)),
            TfmcpToolCall::Output(name) => tfmcp
                .output(name.as_deref())
                .await
                .map(|result| serde_json::json!(result)),
            TfmcpToolCall::Taint(input) => tfmcp
                .taint(&input.action, &input.address)
                .await
                .map(|result| serde_json::json!(result)),
            TfmcpToolCall::Refresh(target) => tfmcp
                .refresh_state(target.as_deref())
                .await
                .map(|result| serde_json::json!(result)),
            TfmcpToolCall::Providers(include_lock) => tfmcp
                .get_providers(include_lock)
                .await
                .map(|result| serde_json::json!(result)),
            TfmcpToolCall::CheckProviderLockfile => tfmcp
                .check_provider_lockfile()
                .await
                .map(|result| serde_json::json!(result)),
            TfmcpToolCall::QualityChecks => tfmcp
                .run_quality_checks()
                .await
                .map(|report| serde_json::json!(report)),
            TfmcpToolCall::InspectStateSafety => tfmcp
                .inspect_state_safety()
                .await
                .map(|inspection| serde_json::json!(inspection)),
            TfmcpToolCall::DetectDriftCandidates => tfmcp
                .detect_drift_candidates()
                .await
                .map(|candidates| serde_json::json!(candidates)),
            TfmcpToolCall::PrepareTerraformChange => tfmcp
                .prepare_terraform_change()
                .await
                .map(|preparation| serde_json::json!(preparation)),
        }
        .map_err(|e| e.to_string())
    }

    async fn run_registry_call(&self, call: RegistryToolCall) -> Result<CallToolResult, McpError> {
        let tool_name = call.tool_name();
        let error_prefix = call.error_prefix();
        self.run_measured_tool_call(tool_name, error_prefix, self.registry_call_value(call))
            .await
    }

    async fn registry_call_value(
        &self,
        call: RegistryToolCall,
    ) -> Result<serde_json::Value, String> {
        match call {
            RegistryToolCall::SearchProviders(query) => self.search_provider_value(query).await,
            RegistryToolCall::ProviderInfo(input) => {
                json_result(
                    self.registry_client
                        .get_provider_info(&input.provider_name, input.namespace.as_deref()),
                    "provider",
                )
                .await
            }
            RegistryToolCall::ProviderDocs(input) => self.provider_docs_value(input).await,
            RegistryToolCall::SearchModules(query) => self.search_module_value(query).await,
            RegistryToolCall::ModuleDetails(input) => self.module_details_value(input).await,
            RegistryToolCall::SearchPolicies(input) => {
                json_result(
                    self.policy_client
                        .search_policies(&input.query, input.provider_filter.as_deref()),
                    "policies",
                )
                .await
            }
            RegistryToolCall::PolicyDetails(input) => self.policy_details_value(input).await,
        }
    }

    async fn search_provider_value(&self, query: String) -> Result<serde_json::Value, String> {
        json_result(self.provider_resolver.search_providers(&query), "providers").await
    }

    async fn provider_docs_value(
        &self,
        input: ProviderDocsInput,
    ) -> Result<serde_json::Value, String> {
        let namespace = input.namespace.as_deref().unwrap_or("hashicorp");
        let data_type = input.data_type.as_deref().unwrap_or("resources");
        json_result(
            self.registry_client.primary.search_docs(
                &input.provider_name,
                namespace,
                &input.service_slug,
                data_type,
            ),
            "documentation",
        )
        .await
    }

    async fn search_module_value(&self, query: String) -> Result<serde_json::Value, String> {
        json_result(
            self.registry_client.primary.search_modules(&query),
            "modules",
        )
        .await
    }

    async fn module_details_value(&self, input: ModuleInput) -> Result<serde_json::Value, String> {
        json_result(
            self.registry_client.primary.get_module_details(
                &input.namespace,
                &input.name,
                &input.provider,
                input.version.as_deref(),
            ),
            "module",
        )
        .await
    }

    async fn policy_details_value(
        &self,
        input: PolicyDetailsInput,
    ) -> Result<serde_json::Value, String> {
        json_result(
            self.policy_client
                .get_policy_details(&input.namespace, &input.name),
            "policy",
        )
        .await
    }

    fn finish_tool_result(
        &self,
        tool_name: &str,
        error_prefix: &str,
        started: Instant,
        result: Result<serde_json::Value, String>,
    ) -> Result<CallToolResult, McpError> {
        let success = result.is_ok();
        metrics::record_tool_call(tool_name, success, started.elapsed());
        match result {
            Ok(value) => json_success(&value),
            Err(e) => Ok(text_error(error_prefix, e)),
        }
    }

    async fn run_measured_tool_call<Fut>(
        &self,
        tool_name: &str,
        error_prefix: &str,
        action: Fut,
    ) -> Result<CallToolResult, McpError>
    where
        Fut: Future<Output = Result<serde_json::Value, String>>,
    {
        logging::info(&format!("Executing {tool_name} tool"));
        let started = Instant::now();
        self.finish_tool_result(tool_name, error_prefix, started, action.await)
    }

    async fn run_tfe_call(&self, call: TfeToolCall) -> Result<CallToolResult, McpError> {
        let (tool_name, error_prefix) = call.metadata();
        logging::info(&format!("Executing {tool_name} tool"));
        let started = Instant::now();
        let requires_audit = call.requires_audit();
        let result = match self.validate_tfe_call(&call) {
            Ok(()) => self.tfe_call_value(call).await,
            Err(error) => Err(error),
        };
        if requires_audit {
            self.audit_tfe_operation(tool_name, &result);
        }
        self.finish_tool_result(tool_name, error_prefix, started, result)
    }

    async fn tfe(&self, call: TfeToolCall, _context: &C) -> R {
        self.run_tfe_call(call).await
    }

    async fn tfe_call_value(&self, call: TfeToolCall) -> Result<serde_json::Value, String> {
        tfe_call_value!(self, call)
    }

    fn validate_tfe_call(&self, call: &TfeToolCall) -> Result<(), String> {
        if self.deployment_controls.organization_allowlist.is_empty() {
            return Ok(());
        }

        let organization = call.organization().ok_or_else(|| {
            "MCP_ORGANIZATION_ALLOWLIST cannot verify account-wide or ID-scoped TFE requests; use an organization-scoped tool or disable remote TFE access".to_string()
        })?;
        if !self.deployment_controls.organization_allowed(organization) {
            return Err(format!(
                "organization '{organization}' is not allowed by MCP_ORGANIZATION_ALLOWLIST"
            ));
        }

        Ok(())
    }

    fn audit_tfe_operation(&self, tool_name: &str, result: &Result<serde_json::Value, String>) {
        let status = self.tfe_client.status();
        let command = vec!["tfe".to_string(), tool_name.to_string()];
        let audit_entry = self.audit_manager.create_audit_entry(
            tool_name,
            &format!("remote:{}", status.address),
            &command,
            result.is_ok(),
            result.as_ref().err().cloned(),
            None,
        );

        if let Err(e) = self.audit_manager.log_audit_entry(audit_entry) {
            logging::error(&format!("Failed to log TFE audit entry: {e}"));
        }
    }

    async fn token_permissions_value(&self) -> Result<serde_json::Value, String> {
        let account = self
            .tfe_client
            .get_token_permissions()
            .await
            .map_err(|e| e.to_string())?;
        Ok(serde_json::json!({
            "client": self.tfe_client.status(),
            "account": account
        }))
    }

    async fn run_details_value(&self, input: TfeRunInput) -> Result<serde_json::Value, String> {
        json_result(self.tfe_client.get_run(&input.run_id), "run").await
    }

    async fn plan_details_value(&self, input: TfePlanInput) -> Result<serde_json::Value, String> {
        json_result(self.tfe_client.get_plan(&input.plan_id), "plan").await
    }

    async fn plan_logs_value(&self, input: TfePlanInput) -> Result<serde_json::Value, String> {
        json_result(self.tfe_client.get_plan_logs(&input.plan_id), "logs").await
    }

    async fn plan_json_output_value(
        &self,
        input: TfePlanInput,
    ) -> Result<serde_json::Value, String> {
        json_result(
            self.tfe_client.get_plan_json_output(&input.plan_id),
            "plan_json_output",
        )
        .await
    }

    async fn apply_details_value(&self, input: TfeApplyInput) -> Result<serde_json::Value, String> {
        json_result(self.tfe_client.get_apply(&input.apply_id), "apply").await
    }

    async fn apply_logs_value(&self, input: TfeApplyInput) -> Result<serde_json::Value, String> {
        json_result(self.tfe_client.get_apply_logs(&input.apply_id), "logs").await
    }

    async fn workspace_details_value(
        &self,
        input: TfeWorkspaceInput,
    ) -> Result<serde_json::Value, String> {
        if let Some(workspace_id) = input.workspace_id {
            return json_result(
                self.tfe_client.get_workspace_by_id(&workspace_id),
                "workspace",
            )
            .await;
        }

        let organization = input.organization.ok_or_else(|| {
            "organization is required when workspace_id is not provided".to_string()
        })?;
        let workspace_name = input.workspace_name.ok_or_else(|| {
            "workspace_name is required when workspace_id is not provided".to_string()
        })?;

        json_result(
            self.tfe_client
                .get_workspace_by_name(&organization, &workspace_name),
            "workspace",
        )
        .await
    }

    // ============ Core Terraform Operations ============

    #[tool(
        description = "List all resources defined in the Terraform project",
        annotations(title = "List Terraform Resources", read_only_hint = true)
    )]
    async fn list_terraform_resources(&self) -> Result<CallToolResult, McpError> {
        self.run_tfmcp_call(TfmcpToolCall::ListResources).await
    }

    #[tool(
        description = "Execute 'terraform plan' and return the output",
        annotations(title = "Get Terraform Plan", read_only_hint = true)
    )]
    async fn get_terraform_plan(&self) -> Result<CallToolResult, McpError> {
        self.run_tfmcp_call(TfmcpToolCall::Plan).await
    }

    #[tool(
        description = "Apply Terraform configuration (WARNING: Makes actual infrastructure changes)",
        annotations(title = "Apply Terraform", destructive_hint = true)
    )]
    async fn apply_terraform(
        &self,
        params: Parameters<AutoApproveInput>,
    ) -> Result<CallToolResult, McpError> {
        self.run_tfmcp_call(TfmcpToolCall::Apply(params.0.auto_approve))
            .await
    }

    #[tool(
        description = "Destroy all Terraform resources (requires TFMCP_ALLOW_DANGEROUS_OPS=true)",
        annotations(title = "Destroy Terraform", destructive_hint = true)
    )]
    async fn destroy_terraform(
        &self,
        params: Parameters<AutoApproveInput>,
    ) -> Result<CallToolResult, McpError> {
        self.run_tfmcp_call(TfmcpToolCall::Destroy(params.0.auto_approve))
            .await
    }

    #[tool(
        description = "Initialize a Terraform project (downloads providers and modules)",
        annotations(
            title = "Initialize Terraform",
            open_world_hint = true,
            idempotent_hint = true
        )
    )]
    async fn init_terraform(&self) -> Result<CallToolResult, McpError> {
        self.run_tfmcp_call(TfmcpToolCall::Init).await
    }

    #[tool(
        description = "Validate Terraform configuration files",
        annotations(title = "Validate Terraform", read_only_hint = true)
    )]
    async fn validate_terraform(&self) -> Result<CallToolResult, McpError> {
        self.run_tfmcp_call(TfmcpToolCall::Validate).await
    }

    #[tool(
        description = "Perform detailed validation with diagnostics and best practice checks",
        annotations(title = "Validate Terraform (Detailed)", read_only_hint = true)
    )]
    async fn validate_terraform_detailed(&self) -> Result<CallToolResult, McpError> {
        self.run_tfmcp_call(TfmcpToolCall::ValidateDetailed).await
    }

    #[tool(
        description = "Get the current Terraform state",
        annotations(title = "Get Terraform State", read_only_hint = true)
    )]
    async fn get_terraform_state(&self) -> Result<CallToolResult, McpError> {
        self.run_tfmcp_call(TfmcpToolCall::State).await
    }

    // ============ Configuration & Analysis ============

    #[tool(
        description = "Analyze Terraform configuration and return detailed analysis including provider version checks",
        annotations(title = "Analyze Terraform", read_only_hint = true)
    )]
    async fn analyze_terraform(&self) -> Result<CallToolResult, McpError> {
        self.run_analysis_call(AnalysisToolCall::Terraform).await
    }

    #[tool(
        description = "Inspect the local Terraform project and summarize directories, modules, and likely entrypoints",
        annotations(title = "Inspect Terraform Project", read_only_hint = true)
    )]
    async fn inspect_terraform_project(&self) -> Result<CallToolResult, McpError> {
        self.run_tfmcp_call(TfmcpToolCall::InspectProject).await
    }

    #[tool(
        description = "Detect likely Terraform root module entrypoints in the local project",
        annotations(title = "Detect Terraform Entrypoints", read_only_hint = true)
    )]
    async fn detect_terraform_entrypoints(&self) -> Result<CallToolResult, McpError> {
        self.run_tfmcp_call(TfmcpToolCall::DetectEntrypoints).await
    }

    #[tool(
        description = "Change the current Terraform project directory",
        annotations(title = "Set Terraform Directory", idempotent_hint = true)
    )]
    async fn set_terraform_directory(
        &self,
        params: Parameters<DirectoryInput>,
    ) -> Result<CallToolResult, McpError> {
        logging::info("Executing set_terraform_directory tool");
        // This is the only tool that needs a write lock
        let mut tfmcp = self.tfmcp.write().await;
        match tfmcp.change_project_directory(params.0.directory.clone()) {
            Ok(()) => {
                let dir = tfmcp.get_project_directory().to_string_lossy().to_string();
                json_success(&serde_json::json!({
                    "success": true,
                    "directory": dir,
                    "message": format!("Changed to: {}", dir)
                }))
            }
            Err(e) => Ok(CallToolResult::error(vec![ContentBlock::text(format!(
                "Failed to change directory: {e}"
            ))])),
        }
    }

    #[tool(
        description = "Get the current security status, policy information, and secret detection scan results",
        annotations(title = "Get Security Status", read_only_hint = true)
    )]
    async fn get_security_status(&self) -> Result<CallToolResult, McpError> {
        logging::info("Executing get_security_status tool");

        // Get environment-based policy settings
        let allow_dangerous = std::env::var("TFMCP_ALLOW_DANGEROUS_OPS")
            .map(|v| v == "true")
            .unwrap_or(false);
        let allow_auto_approve = std::env::var("TFMCP_ALLOW_AUTO_APPROVE")
            .map(|v| v == "true")
            .unwrap_or(false);

        // Run security scan for secret detection and compliance
        let tfmcp = self.tfmcp.read().await;
        let scan_result = tfmcp.run_security_scan().await;

        let (secrets_detected, compliance_score, scan_status) = match scan_result {
            Ok(checks) => {
                let secrets: Vec<_> = checks
                    .hardcoded_secrets
                    .iter()
                    .map(|s| {
                        serde_json::json!({
                            "file": s.file,
                            "line": s.line,
                            "pattern": s.pattern,
                            "severity": s.severity
                        })
                    })
                    .collect();
                (secrets, checks.compliance_score, "completed")
            }
            Err(e) => {
                logging::error(&format!("Security scan failed: {e}"));
                (vec![], 0, "failed")
            }
        };

        json_success(&serde_json::json!({
            "policy": {
                "allow_dangerous_operations": allow_dangerous,
                "allow_auto_approve": allow_auto_approve
            },
            "permissions": {
                "apply": allow_dangerous,
                "destroy": allow_dangerous,
                "init": true,
                "plan": true,
                "validate": true
            },
            "audit_enabled": true,
            "security_scan": {
                "status": scan_status,
                "secrets_detected": secrets_detected,
                "secrets_count": secrets_detected.len(),
                "compliance_score": compliance_score
            }
        }))
    }

    #[tool(
        description = "Analyze module health with cohesion, coupling metrics, and variable quality checks",
        annotations(title = "Analyze Module Health", read_only_hint = true)
    )]
    async fn analyze_module_health(&self) -> Result<CallToolResult, McpError> {
        self.run_analysis_call(AnalysisToolCall::ModuleHealth).await
    }

    #[tool(
        description = "Get the resource dependency graph",
        annotations(title = "Get Resource Dependency Graph", read_only_hint = true)
    )]
    async fn get_resource_dependency_graph(&self) -> Result<CallToolResult, McpError> {
        self.run_tfmcp_call(TfmcpToolCall::DependencyGraph).await
    }

    #[tool(
        description = "Get module refactoring suggestions",
        annotations(title = "Suggest Module Refactoring", read_only_hint = true)
    )]
    async fn suggest_module_refactoring(&self) -> Result<CallToolResult, McpError> {
        self.run_tfmcp_call(TfmcpToolCall::SuggestRefactoring).await
    }

    // ============ Registry Tools ============

    #[tool(
        description = "Search for Terraform providers in the official registry",
        annotations(
            title = "Search Terraform Providers",
            read_only_hint = true,
            open_world_hint = true
        )
    )]
    async fn search_terraform_providers(
        &self,
        params: Parameters<SearchQueryInput>,
    ) -> Result<CallToolResult, McpError> {
        self.run_registry_call(RegistryToolCall::SearchProviders(params.0.query))
            .await
    }

    #[tool(
        description = "Search for Terraform providers in the official registry (HashiCorp-compatible alias)",
        annotations(
            title = "Search Providers",
            read_only_hint = true,
            open_world_hint = true
        )
    )]
    async fn search_providers(
        &self,
        params: Parameters<SearchQueryInput>,
    ) -> Result<CallToolResult, McpError> {
        self.run_registry_call(RegistryToolCall::SearchProviders(params.0.query))
            .await
    }

    #[tool(
        description = "Get detailed information about a specific provider",
        annotations(
            title = "Get Provider Info",
            read_only_hint = true,
            open_world_hint = true
        )
    )]
    async fn get_provider_info(
        &self,
        params: Parameters<ProviderInput>,
    ) -> Result<CallToolResult, McpError> {
        self.run_registry_call(RegistryToolCall::ProviderInfo(params.0))
            .await
    }

    #[tool(
        description = "Get detailed information about a specific provider (HashiCorp-compatible alias)",
        annotations(
            title = "Get Provider Details",
            read_only_hint = true,
            open_world_hint = true
        )
    )]
    async fn get_provider_details(
        &self,
        params: Parameters<ProviderInput>,
    ) -> Result<CallToolResult, McpError> {
        self.run_registry_call(RegistryToolCall::ProviderInfo(params.0))
            .await
    }

    #[tool(
        description = "Get documentation for a specific provider resource or data source",
        annotations(
            title = "Get Provider Docs",
            read_only_hint = true,
            open_world_hint = true
        )
    )]
    async fn get_provider_docs(
        &self,
        params: Parameters<ProviderDocsInput>,
    ) -> Result<CallToolResult, McpError> {
        self.run_registry_call(RegistryToolCall::ProviderDocs(params.0))
            .await
    }

    #[tool(
        description = "Search for Terraform modules in the registry",
        annotations(
            title = "Search Terraform Modules",
            read_only_hint = true,
            open_world_hint = true
        )
    )]
    async fn search_terraform_modules(
        &self,
        params: Parameters<SearchQueryInput>,
    ) -> Result<CallToolResult, McpError> {
        self.run_registry_call(RegistryToolCall::SearchModules(params.0.query))
            .await
    }

    #[tool(
        description = "Search for Terraform modules in the registry (HashiCorp-compatible alias)",
        annotations(
            title = "Search Modules",
            read_only_hint = true,
            open_world_hint = true
        )
    )]
    async fn search_modules(
        &self,
        params: Parameters<SearchQueryInput>,
    ) -> Result<CallToolResult, McpError> {
        self.run_registry_call(RegistryToolCall::SearchModules(params.0.query))
            .await
    }

    #[tool(
        description = "Get detailed information about a specific module",
        annotations(
            title = "Get Module Details",
            read_only_hint = true,
            open_world_hint = true
        )
    )]
    async fn get_module_details(
        &self,
        params: Parameters<ModuleInput>,
    ) -> Result<CallToolResult, McpError> {
        self.run_registry_call(RegistryToolCall::ModuleDetails(params.0))
            .await
    }

    #[tool(
        description = "Get the latest version of a module",
        annotations(
            title = "Get Latest Module Version",
            read_only_hint = true,
            open_world_hint = true
        )
    )]
    async fn get_latest_module_version(
        &self,
        params: Parameters<ModuleVersionInput>,
    ) -> Result<CallToolResult, McpError> {
        logging::info("Executing get_latest_module_version tool");
        match self
            .registry_client
            .primary
            .get_latest_module_version(&params.0.namespace, &params.0.name, &params.0.provider)
            .await
        {
            Ok(version) => json_success(&serde_json::json!({
                "version": version,
                "module_id": format!("{}/{}/{}", params.0.namespace, params.0.name, params.0.provider)
            })),
            Err(e) => Ok(CallToolResult::error(vec![ContentBlock::text(format!(
                "Failed to get latest module version: {e}"
            ))])),
        }
    }

    #[tool(
        description = "Get the latest version of a provider",
        annotations(
            title = "Get Latest Provider Version",
            read_only_hint = true,
            open_world_hint = true
        )
    )]
    async fn get_latest_provider_version(
        &self,
        params: Parameters<ProviderInput>,
    ) -> Result<CallToolResult, McpError> {
        logging::info("Executing get_latest_provider_version tool");
        match self
            .registry_client
            .get_provider_version(&params.0.provider_name, params.0.namespace.as_deref())
            .await
        {
            Ok((version, namespace)) => json_success(&serde_json::json!({
                "version": version,
                "namespace": namespace,
                "provider_id": format!("{}/{}", namespace, params.0.provider_name)
            })),
            Err(e) => Ok(CallToolResult::error(vec![ContentBlock::text(format!(
                "Failed to get latest provider version: {e}"
            ))])),
        }
    }

    // ============ Local Terraform workflow tools ============

    #[tool(
        description = "Analyze terraform plan with risk scoring and recommendations",
        annotations(title = "Analyze Plan", read_only_hint = true)
    )]
    async fn analyze_plan(
        &self,
        params: Parameters<AnalyzePlanInput>,
    ) -> Result<CallToolResult, McpError> {
        self.run_tfmcp_call(TfmcpToolCall::AnalyzePlan(params.0.include_risk))
            .await
    }

    #[tool(
        description = "Review terraform plan with risk, blocker, and recommendation summary",
        annotations(title = "Review Terraform Plan", read_only_hint = true)
    )]
    async fn review_terraform_plan(&self) -> Result<CallToolResult, McpError> {
        self.run_tfmcp_call(TfmcpToolCall::ReviewPlan).await
    }

    #[tool(
        description = "Generate a markdown Terraform plan summary suitable for PR comments",
        annotations(title = "Summarize Plan for PR", read_only_hint = true)
    )]
    async fn summarize_plan_for_pr(&self) -> Result<CallToolResult, McpError> {
        self.run_tfmcp_call(TfmcpToolCall::SummarizePlanForPr).await
    }

    #[tool(
        description = "Analyze terraform state with optional drift detection",
        annotations(title = "Analyze State", read_only_hint = true)
    )]
    async fn analyze_state(
        &self,
        params: Parameters<AnalyzeStateInput>,
    ) -> Result<CallToolResult, McpError> {
        self.run_tfmcp_call(TfmcpToolCall::AnalyzeState {
            resource_type: params.0.resource_type,
            detect_drift: params.0.detect_drift,
        })
        .await
    }

    #[tool(
        description = "Manage terraform workspaces (list, show, new, select, delete)",
        annotations(title = "Terraform Workspace", idempotent_hint = true)
    )]
    async fn terraform_workspace(
        &self,
        params: Parameters<WorkspaceInput>,
    ) -> Result<CallToolResult, McpError> {
        self.run_tfmcp_call(TfmcpToolCall::Workspace {
            action: params.0.action,
            name: params.0.name,
        })
        .await
    }

    #[tool(
        description = "Import existing resources into Terraform state",
        annotations(title = "Terraform Import", destructive_hint = true)
    )]
    async fn terraform_import(
        &self,
        params: Parameters<ImportInput>,
    ) -> Result<CallToolResult, McpError> {
        self.run_tfmcp_call(TfmcpToolCall::Import(params.0)).await
    }

    #[tool(
        description = "Format Terraform configuration files",
        annotations(title = "Terraform Format", idempotent_hint = true)
    )]
    async fn terraform_fmt(
        &self,
        params: Parameters<FmtInput>,
    ) -> Result<CallToolResult, McpError> {
        self.run_tfmcp_call(TfmcpToolCall::Fmt(params.0)).await
    }

    #[tool(
        description = "Generate Terraform dependency graph in DOT format",
        annotations(title = "Terraform Graph", read_only_hint = true)
    )]
    async fn terraform_graph(
        &self,
        params: Parameters<GraphInput>,
    ) -> Result<CallToolResult, McpError> {
        self.run_tfmcp_call(TfmcpToolCall::Graph(params.0.graph_type))
            .await
    }

    #[tool(
        description = "Get Terraform output values",
        annotations(title = "Terraform Output", read_only_hint = true)
    )]
    async fn terraform_output(
        &self,
        params: Parameters<OutputInput>,
    ) -> Result<CallToolResult, McpError> {
        self.run_tfmcp_call(TfmcpToolCall::Output(params.0.name))
            .await
    }

    #[tool(
        description = "Taint or untaint a resource (deprecated: use -replace instead)",
        annotations(title = "Terraform Taint", destructive_hint = true)
    )]
    async fn terraform_taint(
        &self,
        params: Parameters<TaintInput>,
    ) -> Result<CallToolResult, McpError> {
        self.run_tfmcp_call(TfmcpToolCall::Taint(params.0)).await
    }

    #[tool(
        description = "Refresh Terraform state to match real infrastructure",
        annotations(title = "Terraform Refresh", destructive_hint = true)
    )]
    async fn terraform_refresh(
        &self,
        params: Parameters<RefreshInput>,
    ) -> Result<CallToolResult, McpError> {
        self.run_tfmcp_call(TfmcpToolCall::Refresh(params.0.target))
            .await
    }

    #[tool(
        description = "Get information about Terraform providers and lock file",
        annotations(title = "Terraform Providers", read_only_hint = true)
    )]
    async fn terraform_providers(
        &self,
        params: Parameters<ProvidersInput>,
    ) -> Result<CallToolResult, McpError> {
        self.run_tfmcp_call(TfmcpToolCall::Providers(params.0.include_lock))
            .await
    }

    #[tool(
        description = "Check .terraform.lock.hcl for reproducible provider selections",
        annotations(title = "Check Provider Lockfile", read_only_hint = true)
    )]
    async fn check_provider_lockfile(&self) -> Result<CallToolResult, McpError> {
        self.run_tfmcp_call(TfmcpToolCall::CheckProviderLockfile)
            .await
    }

    #[tool(
        description = "Run CI-friendly Terraform quality checks and return JSON plus markdown report output",
        annotations(title = "Run Terraform Quality Checks", read_only_hint = true)
    )]
    async fn run_terraform_quality_checks(&self) -> Result<CallToolResult, McpError> {
        self.run_tfmcp_call(TfmcpToolCall::QualityChecks).await
    }

    #[tool(
        description = "Inspect Terraform state safety, drift risk, provider lockfile status, and change blockers",
        annotations(title = "Inspect State Safety", read_only_hint = true)
    )]
    async fn inspect_state_safety(&self) -> Result<CallToolResult, McpError> {
        self.run_tfmcp_call(TfmcpToolCall::InspectStateSafety).await
    }

    #[tool(
        description = "Detect Terraform drift candidates from readable state without modifying infrastructure",
        annotations(title = "Detect Drift Candidates", read_only_hint = true)
    )]
    async fn detect_drift_candidates(&self) -> Result<CallToolResult, McpError> {
        self.run_tfmcp_call(TfmcpToolCall::DetectDriftCandidates)
            .await
    }

    #[tool(
        description = "Prepare a Terraform change by returning blockers, warnings, and a recommended review sequence",
        annotations(title = "Prepare Terraform Change", read_only_hint = true)
    )]
    async fn prepare_terraform_change(&self) -> Result<CallToolResult, McpError> {
        self.run_tfmcp_call(TfmcpToolCall::PrepareTerraformChange)
            .await
    }

    // ============ HCP Terraform / Terraform Enterprise Read-Only Tools ============

    #[tool(
        description = "Get details about the configured HCP Terraform or Terraform Enterprise token without exposing the token value",
        annotations(
            title = "Get Token Permissions",
            read_only_hint = true,
            open_world_hint = true
        )
    )]
    async fn get_token_permissions(&self, context: C) -> R {
        self.tfe(TfeToolCall::TokenPermissions, &context).await
    }

    #[tool(
        description = "List HCP Terraform or Terraform Enterprise organizations visible to the configured token",
        annotations(
            title = "List Terraform Organizations",
            read_only_hint = true,
            open_world_hint = true
        )
    )]
    async fn list_terraform_orgs(&self, params: P<TfePageInput>, context: C) -> R {
        self.tfe(TfeToolCall::ListOrganizations(params.0), &context)
            .await
    }

    #[tool(
        description = "List HCP Terraform or Terraform Enterprise projects in an organization",
        annotations(
            title = "List Terraform Projects",
            read_only_hint = true,
            open_world_hint = true
        )
    )]
    async fn list_terraform_projects(&self, params: P<TfeOrganizationInput>, context: C) -> R {
        self.tfe(TfeToolCall::ListProjects(params.0), &context)
            .await
    }

    #[tool(
        description = "List HCP Terraform or Terraform Enterprise workspaces in an organization",
        annotations(
            title = "List Workspaces",
            read_only_hint = true,
            open_world_hint = true
        )
    )]
    async fn list_workspaces(&self, params: P<TfeOrganizationInput>, context: C) -> R {
        self.tfe(TfeToolCall::ListWorkspaces(params.0), &context)
            .await
    }

    #[tool(
        description = "Get HCP Terraform or Terraform Enterprise workspace details by workspace ID or organization/name",
        annotations(
            title = "Get Workspace Details",
            read_only_hint = true,
            open_world_hint = true
        )
    )]
    async fn get_workspace_details(&self, params: P<TfeWorkspaceInput>, context: C) -> R {
        self.tfe(TfeToolCall::WorkspaceDetails(params.0), &context)
            .await
    }

    #[tool(
        description = "List HCP Terraform or Terraform Enterprise runs for a workspace",
        annotations(title = "List Runs", read_only_hint = true, open_world_hint = true)
    )]
    async fn list_runs(&self, params: P<TfeWorkspaceRunsInput>, context: C) -> R {
        self.tfe(TfeToolCall::ListRuns(params.0), &context).await
    }

    #[tool(
        description = "Get HCP Terraform or Terraform Enterprise run details",
        annotations(
            title = "Get Run Details",
            read_only_hint = true,
            open_world_hint = true
        )
    )]
    async fn get_run_details(&self, params: P<TfeRunInput>, context: C) -> R {
        self.tfe(TfeToolCall::RunDetails(params.0), &context).await
    }

    #[tool(
        description = "Get HCP Terraform or Terraform Enterprise plan details",
        annotations(
            title = "Get Plan Details",
            read_only_hint = true,
            open_world_hint = true
        )
    )]
    async fn get_plan_details(&self, params: P<TfePlanInput>, context: C) -> R {
        self.tfe(TfeToolCall::PlanDetails(params.0), &context).await
    }

    #[tool(
        description = "Get HCP Terraform or Terraform Enterprise plan logs",
        annotations(title = "Get Plan Logs", read_only_hint = true, open_world_hint = true)
    )]
    async fn get_plan_logs(&self, params: P<TfePlanInput>, context: C) -> R {
        self.tfe(TfeToolCall::PlanLogs(params.0), &context).await
    }

    #[tool(
        description = "Get HCP Terraform or Terraform Enterprise JSON plan output",
        annotations(
            title = "Get Plan JSON Output",
            read_only_hint = true,
            open_world_hint = true
        )
    )]
    async fn get_plan_json_output(&self, params: P<TfePlanInput>, context: C) -> R {
        self.tfe(TfeToolCall::PlanJsonOutput(params.0), &context)
            .await
    }

    #[tool(
        description = "Get HCP Terraform or Terraform Enterprise apply details",
        annotations(
            title = "Get Apply Details",
            read_only_hint = true,
            open_world_hint = true
        )
    )]
    async fn get_apply_details(&self, params: P<TfeApplyInput>, context: C) -> R {
        self.tfe(TfeToolCall::ApplyDetails(params.0), &context)
            .await
    }

    #[tool(
        description = "Get HCP Terraform or Terraform Enterprise apply logs",
        annotations(
            title = "Get Apply Logs",
            read_only_hint = true,
            open_world_hint = true
        )
    )]
    async fn get_apply_logs(&self, params: P<TfeApplyInput>, context: C) -> R {
        self.tfe(TfeToolCall::ApplyLogs(params.0), &context).await
    }

    #[tool(
        description = "Search HCP Terraform or Terraform Enterprise private registry modules",
        annotations(
            title = "Search Private Modules",
            read_only_hint = true,
            open_world_hint = true
        )
    )]
    async fn search_private_modules(
        &self,
        params: P<TfePrivateModuleSearchInput>,
        context: C,
    ) -> R {
        self.tfe(TfeToolCall::SearchPrivateModules(params.0), &context)
            .await
    }

    #[tool(
        description = "Get HCP Terraform or Terraform Enterprise private registry module details",
        annotations(
            title = "Get Private Module Details",
            read_only_hint = true,
            open_world_hint = true
        )
    )]
    async fn get_private_module_details(
        &self,
        params: P<TfePrivateModuleDetailsInput>,
        context: C,
    ) -> R {
        self.tfe(TfeToolCall::PrivateModuleDetails(params.0), &context)
            .await
    }

    #[tool(
        description = "Search HCP Terraform or Terraform Enterprise private registry providers",
        annotations(
            title = "Search Private Providers",
            read_only_hint = true,
            open_world_hint = true
        )
    )]
    async fn search_private_providers(
        &self,
        params: P<TfePrivateProviderSearchInput>,
        context: C,
    ) -> R {
        self.tfe(TfeToolCall::SearchPrivateProviders(params.0), &context)
            .await
    }

    #[tool(
        description = "Get HCP Terraform or Terraform Enterprise private registry provider details",
        annotations(
            title = "Get Private Provider Details",
            read_only_hint = true,
            open_world_hint = true
        )
    )]
    async fn get_private_provider_details(
        &self,
        params: P<TfePrivateProviderDetailsInput>,
        context: C,
    ) -> R {
        self.tfe(TfeToolCall::PrivateProviderDetails(params.0), &context)
            .await
    }

    // ============ HCP Terraform / Terraform Enterprise Gated Operations ============

    #[tool(
        description = "Create an HCP Terraform or Terraform Enterprise workspace. Requires ENABLE_TF_OPERATIONS=true.",
        annotations(
            title = "Create Workspace",
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn create_workspace(&self, params: P<TfeCreateWorkspaceInput>, context: C) -> R {
        self.tfe(TfeToolCall::CreateWorkspace(params.0), &context)
            .await
    }

    #[tool(
        description = "Update an HCP Terraform or Terraform Enterprise workspace. Requires ENABLE_TF_OPERATIONS=true.",
        annotations(
            title = "Update Workspace",
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn update_workspace(&self, params: P<TfeUpdateWorkspaceInput>, context: C) -> R {
        self.tfe(TfeToolCall::UpdateWorkspace(params.0), &context)
            .await
    }

    #[tool(
        description = "Safely delete an HCP Terraform or Terraform Enterprise workspace after remote safety checks. Requires ENABLE_TF_OPERATIONS=true.",
        annotations(
            title = "Delete Workspace Safely",
            destructive_hint = true,
            open_world_hint = true
        )
    )]
    async fn delete_workspace_safely(&self, params: P<TfeWorkspaceRefInput>, context: C) -> R {
        self.tfe(TfeToolCall::DeleteWorkspaceSafely(params.0), &context)
            .await
    }

    #[tool(
        description = "Create an HCP Terraform or Terraform Enterprise run. Requires ENABLE_TF_OPERATIONS=true.",
        annotations(title = "Create Run", idempotent_hint = false, open_world_hint = true)
    )]
    async fn create_run(&self, params: P<TfeCreateRunInput>, context: C) -> R {
        self.tfe(TfeToolCall::CreateRun(params.0), &context).await
    }

    #[tool(
        description = "Apply, discard, cancel, force-cancel, or force-execute an HCP Terraform/TFE run. Requires ENABLE_TF_OPERATIONS=true.",
        annotations(title = "Action Run", destructive_hint = true, open_world_hint = true)
    )]
    async fn action_run(&self, params: P<TfeActionRunInput>, context: C) -> R {
        self.tfe(TfeToolCall::ActionRun(params.0), &context).await
    }

    #[tool(
        description = "List HCP Terraform or Terraform Enterprise workspace variables",
        annotations(
            title = "List Workspace Variables",
            read_only_hint = true,
            open_world_hint = true
        )
    )]
    async fn list_workspace_variables(
        &self,
        params: P<TfeWorkspaceVariablesInput>,
        context: C,
    ) -> R {
        self.tfe(TfeToolCall::ListWorkspaceVariables(params.0), &context)
            .await
    }

    #[tool(
        description = "Create an HCP Terraform or Terraform Enterprise workspace variable. Requires ENABLE_TF_OPERATIONS=true.",
        annotations(
            title = "Create Workspace Variable",
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn create_workspace_variable(
        &self,
        params: P<TfeCreateWorkspaceVariableInput>,
        context: C,
    ) -> R {
        self.tfe(TfeToolCall::CreateWorkspaceVariable(params.0), &context)
            .await
    }

    #[tool(
        description = "Update an HCP Terraform or Terraform Enterprise workspace variable. Requires ENABLE_TF_OPERATIONS=true.",
        annotations(
            title = "Update Workspace Variable",
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn update_workspace_variable(
        &self,
        params: P<TfeUpdateWorkspaceVariableInput>,
        context: C,
    ) -> R {
        self.tfe(TfeToolCall::UpdateWorkspaceVariable(params.0), &context)
            .await
    }

    #[tool(
        description = "Get policy sets attached to an HCP Terraform or Terraform Enterprise workspace",
        annotations(
            title = "Get Workspace Policy Sets",
            read_only_hint = true,
            open_world_hint = true
        )
    )]
    async fn get_workspace_policy_sets(&self, params: PWorkspacePolicySets, ctx: C) -> R {
        self.tfe(TfeToolCall::WorkspacePolicySets(params.0), &ctx)
            .await
    }

    #[tool(
        description = "Attach an HCP Terraform or Terraform Enterprise policy set to a workspace. Requires ENABLE_TF_OPERATIONS=true.",
        annotations(
            title = "Attach Policy Set To Workspace",
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn attach_policy_set_to_workspace(&self, params: PAttachPolicySet, ctx: C) -> R {
        self.tfe(TfeToolCall::AttachPolicySetToWorkspace(params.0), &ctx)
            .await
    }

    #[tool(
        description = "List HCP Terraform or Terraform Enterprise variable sets in an organization",
        annotations(
            title = "List Variable Sets",
            read_only_hint = true,
            open_world_hint = true
        )
    )]
    async fn list_variable_sets(&self, params: PVariableSets, ctx: C) -> R {
        self.tfe(TfeToolCall::ListVariableSets(params.0), &ctx)
            .await
    }

    #[tool(
        description = "Create an HCP Terraform or Terraform Enterprise variable set. Requires ENABLE_TF_OPERATIONS=true.",
        annotations(
            title = "Create Variable Set",
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn create_variable_set(&self, params: PCreateVariableSet, ctx: C) -> R {
        self.tfe(TfeToolCall::CreateVariableSet(params.0), &ctx)
            .await
    }

    #[tool(
        description = "Create a variable inside an HCP Terraform or Terraform Enterprise variable set. Requires ENABLE_TF_OPERATIONS=true.",
        annotations(
            title = "Create Variable In Variable Set",
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn create_variable_in_variable_set(&self, params: PCreateVariableInSet, ctx: C) -> R {
        self.tfe(TfeToolCall::CreateVariableInVariableSet(params.0), &ctx)
            .await
    }

    #[tool(
        description = "Delete a variable from an HCP Terraform or Terraform Enterprise variable set. Requires ENABLE_TF_OPERATIONS=true.",
        annotations(
            title = "Delete Variable In Variable Set",
            destructive_hint = true,
            open_world_hint = true
        )
    )]
    async fn delete_variable_in_variable_set(&self, params: PDeleteVariableInSet, ctx: C) -> R {
        self.tfe(TfeToolCall::DeleteVariableInVariableSet(params.0), &ctx)
            .await
    }

    #[tool(
        description = "Attach an HCP Terraform or Terraform Enterprise variable set to workspaces. Requires ENABLE_TF_OPERATIONS=true.",
        annotations(
            title = "Attach Variable Set To Workspaces",
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn attach_variable_set_to_workspaces(&self, params: PVariableSetWorkspaces, ctx: C) -> R {
        self.tfe(TfeToolCall::AttachVariableSetToWorkspaces(params.0), &ctx)
            .await
    }

    #[tool(
        description = "Detach an HCP Terraform or Terraform Enterprise variable set from workspaces. Requires ENABLE_TF_OPERATIONS=true.",
        annotations(
            title = "Detach Variable Set From Workspaces",
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn detach_variable_set_from_workspaces(
        &self,
        params: PVariableSetWorkspaces,
        ctx: C,
    ) -> R {
        self.tfe(TfeToolCall::DetachVariableSetFromWorkspaces(params.0), &ctx)
            .await
    }

    #[tool(
        description = "Read HCP Terraform or Terraform Enterprise workspace tags",
        annotations(
            title = "Read Workspace Tags",
            read_only_hint = true,
            open_world_hint = true
        )
    )]
    async fn read_workspace_tags(&self, params: PWorkspaceTags, ctx: C) -> R {
        self.tfe(TfeToolCall::ReadWorkspaceTags(params.0), &ctx)
            .await
    }

    #[tool(
        description = "Create or attach HCP Terraform or Terraform Enterprise workspace tags. Requires ENABLE_TF_OPERATIONS=true.",
        annotations(
            title = "Create Workspace Tags",
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn create_workspace_tags(&self, params: PCreateWorkspaceTags, ctx: C) -> R {
        self.tfe(TfeToolCall::CreateWorkspaceTags(params.0), &ctx)
            .await
    }

    #[tool(
        description = "List HCP Terraform or Terraform Enterprise stacks in an organization",
        annotations(title = "List Stacks", read_only_hint = true, open_world_hint = true)
    )]
    async fn list_stacks(&self, params: PStacks, ctx: C) -> R {
        self.tfe(TfeToolCall::ListStacks(params.0), &ctx).await
    }

    #[tool(
        description = "Get HCP Terraform or Terraform Enterprise stack details",
        annotations(
            title = "Get Stack Details",
            read_only_hint = true,
            open_world_hint = true
        )
    )]
    async fn get_stack_details(&self, params: PStack, ctx: C) -> R {
        self.tfe(TfeToolCall::StackDetails(params.0), &ctx).await
    }

    // ============ Terraform inspection and state tools ============

    #[tool(
        description = "Search for Terraform policies (Sentinel/OPA) in the public registry",
        annotations(
            title = "Search Policies",
            read_only_hint = true,
            open_world_hint = true
        )
    )]
    async fn search_policies(
        &self,
        params: Parameters<PolicySearchInput>,
    ) -> Result<CallToolResult, McpError> {
        self.run_registry_call(RegistryToolCall::SearchPolicies(params.0))
            .await
    }

    #[tool(
        description = "Get detailed information about a specific policy library from the registry",
        annotations(
            title = "Get Policy Details",
            read_only_hint = true,
            open_world_hint = true
        )
    )]
    async fn get_policy_details(
        &self,
        params: Parameters<PolicyDetailsInput>,
    ) -> Result<CallToolResult, McpError> {
        self.run_registry_call(RegistryToolCall::PolicyDetails(params.0))
            .await
    }

    #[tool(
        description = "Get provider capabilities: resources, data sources, functions, and guides available",
        annotations(
            title = "Get Provider Capabilities",
            read_only_hint = true,
            open_world_hint = true
        )
    )]
    async fn get_provider_capabilities(
        &self,
        params: Parameters<ProviderCapabilitiesInput>,
    ) -> Result<CallToolResult, McpError> {
        logging::info("Executing get_provider_capabilities tool");
        let namespace = params.0.namespace.as_deref().unwrap_or("hashicorp");
        match self
            .registry_client
            .primary
            .get_provider_info(&params.0.provider_name, namespace)
            .await
        {
            Ok(info) => {
                // Extract docs from the extra fields (API returns docs array)
                let docs = info
                    .extra
                    .get("docs")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();

                // Categorize docs by type
                let mut categories: std::collections::HashMap<String, Vec<String>> =
                    std::collections::HashMap::new();
                for doc in &docs {
                    let cat = doc
                        .get("category")
                        .and_then(|v| v.as_str())
                        .unwrap_or("other")
                        .to_string();
                    let title = doc
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();
                    categories.entry(cat).or_default().push(title);
                }

                let capabilities: serde_json::Value = categories
                    .iter()
                    .map(|(cat, items)| {
                        (
                            cat.clone(),
                            serde_json::json!({
                                "count": items.len(),
                                "items": items.iter().take(20).collect::<Vec<_>>()
                            }),
                        )
                    })
                    .collect();

                json_success(&serde_json::json!({
                    "provider": {
                        "name": info.name,
                        "namespace": namespace,
                        "version": info.version,
                        "description": info.description,
                        "downloads": info.downloads,
                    },
                    "capabilities": capabilities,
                    "total_docs": docs.len()
                }))
            }
            Err(e) => Ok(CallToolResult::error(vec![ContentBlock::text(format!(
                "Provider capabilities failed: {e}"
            ))])),
        }
    }
}
