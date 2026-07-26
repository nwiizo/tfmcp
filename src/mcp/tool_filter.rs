use crate::shared::logging;
use std::collections::HashSet;

/// Tool filtering configuration.
#[derive(Clone, Debug)]
pub struct ToolFilter {
    enabled_tools: Option<HashSet<String>>,
}

impl ToolFilter {
    /// Create a filter that enables all tools.
    pub fn all() -> Self {
        Self {
            enabled_tools: None,
        }
    }

    /// Create a filter from toolset categories and optional individual tool list.
    pub fn from_cli(toolsets: &[String], tools: Option<&[String]>) -> Self {
        if let Some(tool_list) = tools {
            return Self {
                enabled_tools: Some(tool_list.iter().map(|s| s.to_string()).collect()),
            };
        }

        let mut enabled = HashSet::new();
        let mut recognized_toolset = false;
        for toolset in toolsets {
            match toolset.as_str() {
                "all" => return Self::all(),
                "terraform" => {
                    recognized_toolset = true;
                    add_tools(&mut enabled, TOOLSET_TERRAFORM);
                }
                "registry" => {
                    recognized_toolset = true;
                    add_tools(&mut enabled, TOOLSET_REGISTRY);
                }
                "registry-private" => {
                    recognized_toolset = true;
                    add_tools(&mut enabled, TOOLSET_REGISTRY_PRIVATE);
                }
                "default" => {
                    recognized_toolset = true;
                    add_tools(&mut enabled, TOOLSET_DEFAULT);
                }
                "analysis" => {
                    recognized_toolset = true;
                    add_tools(&mut enabled, TOOLSET_ANALYSIS);
                }
                "operations" => {
                    recognized_toolset = true;
                    add_tools(&mut enabled, TOOLSET_OPERATIONS);
                }
                _ => {
                    logging::error(&format!("Unknown toolset: {toolset}"));
                }
            }
        }

        if enabled.is_empty() && !recognized_toolset && toolsets.is_empty() {
            Self::all()
        } else {
            Self {
                enabled_tools: Some(enabled),
            }
        }
    }

    /// Check if a tool is enabled.
    pub fn is_enabled(&self, tool_name: &str) -> bool {
        match &self.enabled_tools {
            None => true,
            Some(set) => set.contains(tool_name),
        }
    }
}

fn add_tools(enabled: &mut HashSet<String>, tools: &[&str]) {
    enabled.extend(tools.iter().map(|name| name.to_string()));
}

const TOOLSET_TERRAFORM: &[&str] = &[
    "init_terraform",
    "get_terraform_plan",
    "apply_terraform",
    "destroy_terraform",
    "validate_terraform",
    "validate_terraform_detailed",
    "get_terraform_state",
    "inspect_terraform_project",
    "detect_terraform_entrypoints",
    "list_terraform_resources",
    "set_terraform_directory",
    "terraform_workspace",
    "terraform_fmt",
    "terraform_graph",
    "terraform_output",
    "terraform_providers",
    "run_terraform_quality_checks",
    "inspect_state_safety",
    "detect_drift_candidates",
    "prepare_terraform_change",
    "terraform_import",
    "terraform_taint",
    "terraform_refresh",
    "get_token_permissions",
    "list_terraform_orgs",
    "list_terraform_projects",
    "list_workspaces",
    "get_workspace_details",
    "list_runs",
    "get_run_details",
    "get_plan_details",
    "get_plan_logs",
    "get_plan_json_output",
    "get_apply_details",
    "get_apply_logs",
    "get_workspace_policy_sets",
    "list_workspace_variables",
    "list_variable_sets",
    "read_workspace_tags",
    "list_stacks",
    "get_stack_details",
];

const TOOLSET_REGISTRY: &[&str] = &[
    "search_providers",
    "search_terraform_providers",
    "get_provider_details",
    "get_provider_info",
    "get_provider_docs",
    "get_provider_capabilities",
    "search_modules",
    "search_terraform_modules",
    "get_module_details",
    "get_latest_module_version",
    "get_latest_provider_version",
    "search_policies",
    "get_policy_details",
    "search_private_modules",
    "get_private_module_details",
    "search_private_providers",
    "get_private_provider_details",
];

const TOOLSET_REGISTRY_PRIVATE: &[&str] = &[
    "search_private_modules",
    "get_private_module_details",
    "search_private_providers",
    "get_private_provider_details",
];

const TOOLSET_ANALYSIS: &[&str] = &[
    "analyze_terraform",
    "analyze_module_health",
    "get_resource_dependency_graph",
    "suggest_module_refactoring",
    "get_security_status",
    "analyze_plan",
    "review_terraform_plan",
    "summarize_plan_for_pr",
    "check_provider_lockfile",
    "run_terraform_quality_checks",
    "inspect_state_safety",
    "detect_drift_candidates",
    "prepare_terraform_change",
    "analyze_state",
    "inspect_terraform_project",
    "detect_terraform_entrypoints",
];

const TOOLSET_OPERATIONS: &[&str] = &[
    "create_workspace",
    "update_workspace",
    "delete_workspace_safely",
    "create_run",
    "action_run",
    "create_workspace_variable",
    "update_workspace_variable",
    "attach_policy_set_to_workspace",
    "create_variable_set",
    "create_variable_in_variable_set",
    "delete_variable_in_variable_set",
    "attach_variable_set_to_workspaces",
    "detach_variable_set_from_workspaces",
    "create_workspace_tags",
];

const TOOLSET_DEFAULT: &[&str] = &[
    "search_providers",
    "get_provider_details",
    "get_provider_capabilities",
    "get_latest_provider_version",
    "search_modules",
    "get_module_details",
    "get_latest_module_version",
    "search_policies",
    "get_policy_details",
    "inspect_terraform_project",
    "detect_terraform_entrypoints",
    "analyze_terraform",
    "analyze_module_health",
    "review_terraform_plan",
    "summarize_plan_for_pr",
    "check_provider_lockfile",
    "run_terraform_quality_checks",
    "inspect_state_safety",
    "detect_drift_candidates",
    "prepare_terraform_change",
    "validate_terraform",
    "validate_terraform_detailed",
    "get_security_status",
    "get_token_permissions",
    "list_terraform_orgs",
    "list_terraform_projects",
    "list_workspaces",
    "get_workspace_details",
    "list_runs",
    "get_run_details",
    "get_plan_details",
    "get_apply_details",
    "get_workspace_policy_sets",
    "list_workspace_variables",
    "list_variable_sets",
    "read_workspace_tags",
    "list_stacks",
    "get_stack_details",
    "search_private_modules",
    "get_private_module_details",
    "search_private_providers",
    "get_private_provider_details",
];
