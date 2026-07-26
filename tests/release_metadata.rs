use serde_json::Value;
use tfmcp::mcp::server::ToolFilter;

#[test]
fn server_json_matches_mcp_registry_oci_requirements() {
    let metadata: Value = serde_json::from_str(include_str!("../server.json"))
        .expect("server.json must be valid JSON");

    assert_eq!(
        metadata["$schema"],
        "https://static.modelcontextprotocol.io/schemas/2025-12-11/server.schema.json"
    );
    assert_eq!(metadata["name"], "io.github.nwiizo/tfmcp");
    assert_eq!(metadata["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(
        metadata["repository"]["url"],
        "https://github.com/nwiizo/tfmcp"
    );
    assert_eq!(metadata["repository"]["source"], "github");

    let packages = metadata["packages"]
        .as_array()
        .expect("server.json packages must be an array");
    assert_eq!(packages.len(), 1);

    let package = &packages[0];
    assert_eq!(package["registryType"], "oci");
    assert_eq!(
        package["identifier"],
        format!("ghcr.io/nwiizo/tfmcp:{}", env!("CARGO_PKG_VERSION"))
    );
    assert_eq!(package["transport"]["type"], "stdio");

    let env_vars = package["environmentVariables"]
        .as_array()
        .expect("server.json package environmentVariables must be an array");
    let tfe_token = env_vars
        .iter()
        .find(|env| env["name"] == "TFE_TOKEN")
        .expect("TFE_TOKEN must be documented in server.json");
    assert_eq!(tfe_token["isSecret"], true);

    for name in [
        "ENABLE_TF_OPERATIONS",
        "TFE_MAX_RESPONSE_BYTES",
        "TRANSPORT_MODE",
        "TRANSPORT_HOST",
        "TRANSPORT_PORT",
        "MCP_ENDPOINT",
        "MCP_HEALTH_ENDPOINT",
        "MCP_METRICS_ENDPOINT",
        "MCP_SESSION_MODE",
        "MCP_CORS_MODE",
        "MCP_ALLOWED_ORIGINS",
        "MCP_ALLOWED_HOSTS",
        "MCP_HEARTBEAT_INTERVAL",
        "MCP_ORGANIZATION_ALLOWLIST",
        "MCP_RATE_LIMIT_GLOBAL",
        "MCP_RATE_LIMIT_SESSION",
        "MCP_TLS_CERT_FILE",
        "MCP_TLS_KEY_FILE",
    ] {
        assert!(
            env_vars.iter().any(|env| env["name"] == name),
            "{name} must be documented in server.json"
        );
    }
}

#[test]
fn dockerfile_declares_matching_mcp_registry_label() {
    let dockerfile = include_str!("../Dockerfile");
    assert!(
        dockerfile
            .contains(r#"LABEL io.modelcontextprotocol.server.name="io.github.nwiizo/tfmcp""#),
        "Dockerfile must declare the OCI ownership label required by the MCP Registry"
    );
}

#[test]
fn dockerfile_declares_matching_oci_release_metadata() {
    let dockerfile = include_str!("../Dockerfile");

    assert!(
        dockerfile.contains(&format!(
            r#"ARG TFMCP_VERSION="{}""#,
            env!("CARGO_PKG_VERSION")
        )),
        "Dockerfile TFMCP_VERSION must match Cargo package version"
    );

    for required_label in [
        r#"org.opencontainers.image.title="tfmcp""#,
        r#"org.opencontainers.image.source="https://github.com/nwiizo/tfmcp""#,
        r#"org.opencontainers.image.version="${TFMCP_VERSION}""#,
        r#"org.opencontainers.image.revision="${TFMCP_REVISION}""#,
    ] {
        assert!(
            dockerfile.contains(required_label),
            "Dockerfile must declare OCI label {required_label}"
        );
    }
}

#[test]
fn default_toolset_exposes_release_baseline_without_write_operations() {
    let toolsets = vec!["default".to_string()];
    let filter = ToolFilter::from_cli(&toolsets, None);

    for tool in [
        "search_providers",
        "get_provider_details",
        "get_provider_capabilities",
        "get_latest_provider_version",
        "search_modules",
        "get_module_details",
        "get_latest_module_version",
        "search_policies",
        "get_policy_details",
        "search_private_modules",
        "get_private_module_details",
        "search_private_providers",
        "get_private_provider_details",
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
    ] {
        assert!(
            filter.is_enabled(tool),
            "{tool} should be in default toolset"
        );
    }

    for tool in [
        "apply_terraform",
        "destroy_terraform",
        "terraform_import",
        "terraform_taint",
        "terraform_refresh",
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
    ] {
        assert!(
            !filter.is_enabled(tool),
            "{tool} should not be in default toolset"
        );
    }
}

#[test]
fn operations_toolset_exposes_only_gated_remote_operations() {
    let toolsets = vec!["operations".to_string()];
    let filter = ToolFilter::from_cli(&toolsets, None);

    for tool in [
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
    ] {
        assert!(
            filter.is_enabled(tool),
            "{tool} should be in operations toolset"
        );
    }

    assert!(!filter.is_enabled("apply_terraform"));
    assert!(!filter.is_enabled("search_providers"));
    assert!(!filter.is_enabled("list_workspace_variables"));
}

#[test]
fn terraform_toolset_exposes_hashicorp_v1_read_surface_and_tfmcp_strengths() {
    let toolsets = vec!["terraform".to_string()];
    let filter = ToolFilter::from_cli(&toolsets, None);

    for tool in [
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
        "inspect_terraform_project",
        "detect_terraform_entrypoints",
        "run_terraform_quality_checks",
        "inspect_state_safety",
        "detect_drift_candidates",
        "prepare_terraform_change",
    ] {
        assert!(
            filter.is_enabled(tool),
            "{tool} should be in terraform toolset"
        );
    }

    assert!(!filter.is_enabled("search_providers"));
    assert!(!filter.is_enabled("create_workspace"));
    assert!(!filter.is_enabled("action_run"));
}

#[test]
fn registry_toolset_keeps_hashicorp_aliases_and_tfmcp_names() {
    let toolsets = vec!["registry".to_string()];
    let filter = ToolFilter::from_cli(&toolsets, None);

    for tool in [
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
    ] {
        assert!(
            filter.is_enabled(tool),
            "{tool} should be in registry toolset"
        );
    }

    assert!(!filter.is_enabled("apply_terraform"));
    assert!(!filter.is_enabled("inspect_terraform_project"));
}

#[test]
fn registry_private_toolset_matches_hashicorp_name_without_removing_registry_behavior() {
    let toolsets = vec!["registry-private".to_string()];
    let filter = ToolFilter::from_cli(&toolsets, None);

    for tool in [
        "search_private_modules",
        "get_private_module_details",
        "search_private_providers",
        "get_private_provider_details",
    ] {
        assert!(
            filter.is_enabled(tool),
            "{tool} should be in registry-private toolset"
        );
    }

    assert!(!filter.is_enabled("search_providers"));
    assert!(!filter.is_enabled("list_workspaces"));
}

#[test]
fn explicit_tools_override_toolsets() {
    let toolsets = vec!["all".to_string()];
    let tools = vec!["validate_terraform".to_string()];
    let filter = ToolFilter::from_cli(&toolsets, Some(&tools));

    assert!(filter.is_enabled("validate_terraform"));
    assert!(!filter.is_enabled("search_providers"));
    assert!(!filter.is_enabled("apply_terraform"));
}

#[test]
fn unknown_toolset_does_not_fall_back_to_all_tools() {
    let toolsets = vec!["regsitry".to_string()];
    let filter = ToolFilter::from_cli(&toolsets, None);

    assert!(!filter.is_enabled("search_providers"));
    assert!(!filter.is_enabled("apply_terraform"));
}

#[test]
fn changelog_documents_hashicorp_compatibility_without_erasing_tfmcp_strengths() {
    let release_notes = include_str!("../CHANGELOG.md");

    for expected in [
        "## HashiCorp v1.0.x Compatibility",
        "registry-private",
        "get_plan_json_output",
        "get_apply_logs",
        "list_variable_sets",
        "attach_variable_set_to_workspaces",
        "read_workspace_tags",
        "list_stacks",
        "/terraform/providers/{namespace}/name/{name}/version/{version}",
        "local Terraform CLI workflows",
        "entrypoint detection",
        "module health analysis",
        "state safety checks",
        "local dangerous-operation gates",
    ] {
        assert!(
            release_notes.contains(expected),
            "changelog must document {expected}"
        );
    }
}
