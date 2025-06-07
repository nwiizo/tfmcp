use crate::prompts::builder::{ToolDescription, ToolExample};
use serde_json::json;

/// Create tool description for terraform plan operation
pub fn create_terraform_plan_description() -> ToolDescription {
    ToolDescription::new(
        "Execute 'terraform plan' to show changes that would be made to infrastructure"
    )
    .with_usage_guide(
        "This tool generates an execution plan showing what Terraform will do when you apply \
        the configuration. It's a safe operation that doesn't make any changes to real infrastructure."
    )
    .with_constraint("Terraform must be initialized in the target directory")
    .with_constraint("Valid Terraform configuration files must exist")
    .with_error_hint("Init Required", "Run terraform init first to initialize the working directory")
    .with_error_hint("No Configuration", "Ensure .tf files exist in the project directory")
    .with_security_note("This is a read-only operation - no infrastructure changes are made")
    .with_example(ToolExample {
        title: "Basic Plan Generation".to_string(),
        description: "Generate a plan for the current Terraform configuration".to_string(),
        input: json!({}),
        expected_output: "JSON-formatted plan showing resources to be created, modified, or destroyed".to_string(),
    })
}

/// Create tool description for terraform apply operation
pub fn create_terraform_apply_description() -> ToolDescription {
    ToolDescription::new(
        "Apply Terraform configuration to create, update, or delete infrastructure resources"
    )
    .with_usage_guide(
        "This tool executes the changes shown in a terraform plan. It will modify real infrastructure \
        according to your configuration. Always review the plan before applying changes."
    )
    .with_constraint("TFMCP_ALLOW_DANGEROUS_OPS must be set to true")
    .with_constraint("Terraform must be initialized")
    .with_constraint("Valid configuration files must exist")
    .with_error_hint("Permission Denied", "Set TFMCP_ALLOW_DANGEROUS_OPS=true to enable apply operations")
    .with_error_hint("Init Required", "Run terraform init first")
    .with_error_hint("Auto-approve Blocked", "Set TFMCP_ALLOW_AUTO_APPROVE=true for auto-approval")
    .with_security_note("This operation modifies real infrastructure - use with caution")
    .with_security_note("All apply operations are logged for audit purposes")
    .with_security_note("Production directory patterns are automatically blocked")
    .with_example(ToolExample {
        title: "Apply with Manual Approval".to_string(),
        description: "Apply changes with interactive approval".to_string(),
        input: json!({"auto_approve": false}),
        expected_output: "Terraform apply output showing resources created/modified".to_string(),
    })
    .with_example(ToolExample {
        title: "Auto-approved Apply".to_string(),
        description: "Apply changes automatically without manual confirmation".to_string(),
        input: json!({"auto_approve": true}),
        expected_output: "Terraform apply output with automatic approval".to_string(),
    })
}

/// Create tool description for terraform validate operation
pub fn create_terraform_validate_description() -> ToolDescription {
    ToolDescription::new(
        "Validate Terraform configuration files for syntax and semantic correctness"
    )
    .with_usage_guide(
        "This tool checks your Terraform configuration for syntax errors, missing required \
        arguments, and other validation issues. It's a safe operation that doesn't access remote state."
    )
    .with_constraint("Terraform configuration files must exist in the directory")
    .with_error_hint("No Configuration", "Ensure .tf files exist in the project directory")
    .with_error_hint("Syntax Error", "Check Terraform configuration syntax using terraform fmt")
    .with_security_note("This is a safe, local operation that doesn't modify anything")
    .with_example(ToolExample {
        title: "Basic Validation".to_string(),
        description: "Validate the current Terraform configuration".to_string(),
        input: json!({}),
        expected_output: "Validation result with success status and any error messages".to_string(),
    })
}

/// Create tool description for terraform validate detailed operation
pub fn create_terraform_validate_detailed_description() -> ToolDescription {
    ToolDescription::new(
        "Perform comprehensive validation with best practice analysis and detailed diagnostics",
    )
    .with_usage_guide(
        "This tool performs detailed validation including Terraform's built-in checks plus \
        additional best practice recommendations. It provides detailed diagnostics with file \
        locations and actionable suggestions.",
    )
    .with_constraint("Terraform configuration files must exist")
    .with_error_hint("No Configuration", "Add .tf files to the project directory")
    .with_error_hint(
        "Best Practice Violation",
        "Review suggestions to improve configuration quality",
    )
    .with_security_note("Includes security best practice checks")
    .with_security_note("No infrastructure access required - purely local analysis")
    .with_example(ToolExample {
        title: "Comprehensive Analysis".to_string(),
        description: "Get detailed validation with best practices".to_string(),
        input: json!({}),
        expected_output: "Detailed report with errors, warnings, suggestions, and file locations"
            .to_string(),
    })
}

/// Create tool description for terraform destroy operation  
pub fn create_terraform_destroy_description() -> ToolDescription {
    ToolDescription::new(
        "Destroy all resources defined in the Terraform configuration"
    )
    .with_usage_guide(
        "This tool destroys all infrastructure resources managed by Terraform in the current \
        configuration. This is a destructive operation that cannot be undone. Use with extreme caution."
    )
    .with_constraint("TFMCP_ALLOW_DANGEROUS_OPS must be set to true")
    .with_constraint("Terraform must be initialized")
    .with_constraint("State file must exist with managed resources")
    .with_error_hint("Permission Denied", "Set TFMCP_ALLOW_DANGEROUS_OPS=true to enable destroy operations")
    .with_error_hint("No State", "No Terraform state found - nothing to destroy")
    .with_error_hint("Auto-approve Blocked", "Set TFMCP_ALLOW_AUTO_APPROVE=true for auto-approval")
    .with_security_note("⚠️ DESTRUCTIVE OPERATION - destroys real infrastructure")
    .with_security_note("All destroy operations are logged for audit purposes")
    .with_security_note("Production directory patterns are automatically blocked")
    .with_security_note("Consider backing up important data before destruction")
    .with_example(ToolExample {
        title: "Destroy with Confirmation".to_string(),
        description: "Destroy resources with manual confirmation".to_string(),
        input: json!({"auto_approve": false}),
        expected_output: "Terraform destroy output showing resources removed".to_string(),
    })
}

/// Create tool description for analyzing terraform configurations
pub fn create_terraform_analyze_description() -> ToolDescription {
    ToolDescription::new(
        "Analyze Terraform configuration files to extract detailed information about resources, variables, and providers"
    )
    .with_usage_guide(
        "This tool parses your Terraform configuration files and provides comprehensive analysis \
        including resources, variables, outputs, and provider information. Useful for understanding \
        configuration structure and dependencies."
    )
    .with_constraint("Terraform configuration files must exist in the directory")
    .with_error_hint("No Configuration", "Add .tf files to the project directory to analyze")
    .with_error_hint("Parse Error", "Check Terraform syntax - configuration files may be malformed")
    .with_security_note("Analysis is performed locally without accessing remote resources")
    .with_example(ToolExample {
        title: "Analyze Current Configuration".to_string(),
        description: "Analyze all .tf files in the current directory".to_string(),
        input: json!({}),
        expected_output: "Structured analysis showing resources, variables, outputs, and providers".to_string(),
    })
    .with_example(ToolExample {
        title: "Analyze Specific Path".to_string(),
        description: "Analyze configuration in a specific directory".to_string(),
        input: json!({"path": "/path/to/terraform/config"}),
        expected_output: "Analysis results for the specified directory".to_string(),
    })
}

/// Create tool description for listing terraform resources
pub fn create_list_resources_description() -> ToolDescription {
    ToolDescription::new("List all resources currently managed by Terraform in the state file")
        .with_usage_guide(
            "This tool shows all infrastructure resources that are currently being managed by \
        Terraform according to the state file. Useful for understanding what resources exist \
        and their identifiers.",
        )
        .with_constraint("Terraform must be initialized")
        .with_constraint("State file must exist")
        .with_error_hint(
            "No State",
            "Run terraform apply to create managed resources first",
        )
        .with_error_hint(
            "Init Required",
            "Initialize Terraform working directory first",
        )
        .with_security_note("Reads from local state file - no remote access required")
        .with_example(ToolExample {
            title: "List All Resources".to_string(),
            description: "Show all resources in the current state".to_string(),
            input: json!({}),
            expected_output: "Array of resource identifiers managed by Terraform".to_string(),
        })
}

/// Create tool description for getting terraform state
pub fn create_get_state_description() -> ToolDescription {
    ToolDescription::new("Retrieve the current Terraform state information")
        .with_usage_guide(
            "This tool provides access to the current Terraform state, showing the real-world \
        resources that Terraform is managing and their current configuration.",
        )
        .with_constraint("Terraform must be initialized")
        .with_constraint("State file must exist")
        .with_error_hint(
            "No State",
            "No state file found - apply configuration first",
        )
        .with_error_hint(
            "Corrupted State",
            "State file may be corrupted - check terraform state list",
        )
        .with_security_note("State may contain sensitive information")
        .with_security_note("Read-only operation - state is not modified")
        .with_example(ToolExample {
            title: "Get Current State".to_string(),
            description: "Retrieve the complete Terraform state".to_string(),
            input: json!({}),
            expected_output: "Terraform state information including resource details".to_string(),
        })
}

/// Create tool description for initializing terraform
pub fn create_init_terraform_description() -> ToolDescription {
    ToolDescription::new("Initialize a Terraform working directory and download required providers")
        .with_usage_guide(
            "This tool initializes a Terraform working directory by downloading and installing \
        provider plugins, modules, and setting up the backend. This is typically the first \
        command to run in a new Terraform configuration.",
        )
        .with_constraint("Terraform configuration files must exist")
        .with_constraint("Network access required for downloading providers")
        .with_error_hint(
            "No Configuration",
            "Create .tf files with provider configuration first",
        )
        .with_error_hint(
            "Network Error",
            "Check internet connectivity for provider downloads",
        )
        .with_error_hint(
            "Backend Error",
            "Verify backend configuration if using remote state",
        )
        .with_security_note("Downloads providers from trusted Terraform Registry")
        .with_security_note("May create local state file containing infrastructure information")
        .with_example(ToolExample {
            title: "Initialize Working Directory".to_string(),
            description: "Set up Terraform environment for the current configuration".to_string(),
            input: json!({}),
            expected_output: "Initialization results showing downloaded providers and modules"
                .to_string(),
        })
}

/// Create tool description for setting terraform directory
pub fn create_set_directory_description() -> ToolDescription {
    ToolDescription::new("Change the active Terraform project directory for subsequent operations")
        .with_usage_guide(
            "This tool allows you to switch between different Terraform projects by changing \
        the working directory. All subsequent Terraform operations will use the new directory.",
        )
        .with_constraint("Target directory must exist or be creatable")
        .with_constraint("Directory path must be valid")
        .with_error_hint(
            "Invalid Path",
            "Ensure the directory path exists and is accessible",
        )
        .with_error_hint(
            "Permission Error",
            "Check read/write permissions for the target directory",
        )
        .with_security_note("Automatically creates sample project if no .tf files exist")
        .with_security_note("Directory changes are logged for audit purposes")
        .with_example(ToolExample {
            title: "Switch to Project Directory".to_string(),
            description: "Change to a specific Terraform project".to_string(),
            input: json!({"directory": "/path/to/terraform/project"}),
            expected_output: "Confirmation of directory change with current path".to_string(),
        })
}

/// Create tool description for getting security status
pub fn create_security_status_description() -> ToolDescription {
    ToolDescription::new("Get current security policy configuration and operational permissions")
        .with_usage_guide(
            "This tool provides information about the current security settings, including \
        which operations are allowed, audit logging status, and security policy configuration.",
        )
        .with_security_note("Shows current security policy without exposing sensitive data")
        .with_security_note("Helps understand why certain operations might be blocked")
        .with_example(ToolExample {
            title: "Check Security Configuration".to_string(),
            description: "Review current security settings and permissions".to_string(),
            input: json!({}),
            expected_output: "Security policy details, permissions, and audit status".to_string(),
        })
}

/// Get all improved tool descriptions
pub fn get_all_tool_descriptions() -> std::collections::HashMap<String, ToolDescription> {
    let mut descriptions = std::collections::HashMap::new();

    descriptions.insert(
        "get_terraform_plan".to_string(),
        create_terraform_plan_description(),
    );
    descriptions.insert(
        "apply_terraform".to_string(),
        create_terraform_apply_description(),
    );
    descriptions.insert(
        "validate_terraform".to_string(),
        create_terraform_validate_description(),
    );
    descriptions.insert(
        "validate_terraform_detailed".to_string(),
        create_terraform_validate_detailed_description(),
    );
    descriptions.insert(
        "destroy_terraform".to_string(),
        create_terraform_destroy_description(),
    );
    descriptions.insert(
        "analyze_terraform".to_string(),
        create_terraform_analyze_description(),
    );
    descriptions.insert(
        "list_terraform_resources".to_string(),
        create_list_resources_description(),
    );
    descriptions.insert(
        "get_terraform_state".to_string(),
        create_get_state_description(),
    );
    descriptions.insert(
        "init_terraform".to_string(),
        create_init_terraform_description(),
    );
    descriptions.insert(
        "set_terraform_directory".to_string(),
        create_set_directory_description(),
    );
    descriptions.insert(
        "get_security_status".to_string(),
        create_security_status_description(),
    );

    descriptions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_descriptions_created() {
        let descriptions = get_all_tool_descriptions();

        // Verify all expected tools have descriptions
        assert!(descriptions.contains_key("get_terraform_plan"));
        assert!(descriptions.contains_key("apply_terraform"));
        assert!(descriptions.contains_key("validate_terraform"));
        assert!(descriptions.contains_key("destroy_terraform"));

        // Verify descriptions have content
        for (name, desc) in descriptions {
            assert!(!desc.summary.is_empty(), "Tool {} missing summary", name);
        }
    }

    #[test]
    fn test_security_notes_present() {
        let apply_desc = create_terraform_apply_description();
        assert!(!apply_desc.security_notes.is_empty());

        let destroy_desc = create_terraform_destroy_description();
        assert!(!destroy_desc.security_notes.is_empty());
    }

    #[test]
    fn test_examples_present() {
        let apply_desc = create_terraform_apply_description();
        assert!(!apply_desc.examples.is_empty());

        let analyze_desc = create_terraform_analyze_description();
        assert!(!analyze_desc.examples.is_empty());
    }
}
