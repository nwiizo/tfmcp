use crate::prompts::builder::{ToolDescription, ToolExample};
use serde_json::json;

enum ToolDescriptionKind {
    TerraformPlan,
    TerraformApply,
    TerraformValidate,
    TerraformValidateDetailed,
    TerraformDestroy,
    TerraformAnalyze,
    ListResources,
    GetState,
    InitTerraform,
    SetDirectory,
    SecurityStatus,
}

struct DescriptionSpec {
    summary: &'static str,
    usage_guide: &'static str,
    constraints: Vec<&'static str>,
    error_hints: Vec<(&'static str, &'static str)>,
    security_notes: Vec<&'static str>,
    examples: Vec<ToolExample>,
}

/// Create tool description for terraform plan operation
pub fn create_terraform_plan_description() -> ToolDescription {
    create_description(ToolDescriptionKind::TerraformPlan)
}

/// Create tool description for terraform apply operation
pub fn create_terraform_apply_description() -> ToolDescription {
    create_description(ToolDescriptionKind::TerraformApply)
}

/// Create tool description for terraform validate operation
pub fn create_terraform_validate_description() -> ToolDescription {
    create_description(ToolDescriptionKind::TerraformValidate)
}

/// Create tool description for terraform validate detailed operation
pub fn create_terraform_validate_detailed_description() -> ToolDescription {
    create_description(ToolDescriptionKind::TerraformValidateDetailed)
}

/// Create tool description for terraform destroy operation
pub fn create_terraform_destroy_description() -> ToolDescription {
    create_description(ToolDescriptionKind::TerraformDestroy)
}

/// Create tool description for analyzing terraform configurations
pub fn create_terraform_analyze_description() -> ToolDescription {
    create_description(ToolDescriptionKind::TerraformAnalyze)
}

/// Create tool description for listing terraform resources
pub fn create_list_resources_description() -> ToolDescription {
    create_description(ToolDescriptionKind::ListResources)
}

/// Create tool description for getting terraform state
pub fn create_get_state_description() -> ToolDescription {
    create_description(ToolDescriptionKind::GetState)
}

/// Create tool description for initializing terraform
pub fn create_init_terraform_description() -> ToolDescription {
    create_description(ToolDescriptionKind::InitTerraform)
}

/// Create tool description for setting terraform directory
pub fn create_set_directory_description() -> ToolDescription {
    create_description(ToolDescriptionKind::SetDirectory)
}

/// Create tool description for getting security status
pub fn create_security_status_description() -> ToolDescription {
    create_description(ToolDescriptionKind::SecurityStatus)
}

fn create_description(kind: ToolDescriptionKind) -> ToolDescription {
    build_description(description_spec(kind))
}

fn build_description(spec: DescriptionSpec) -> ToolDescription {
    let mut description = ToolDescription::new(spec.summary).with_usage_guide(spec.usage_guide);

    for constraint in spec.constraints {
        description = description.with_constraint(constraint);
    }

    for (error, hint) in spec.error_hints {
        description = description.with_error_hint(error, hint);
    }

    for note in spec.security_notes {
        description = description.with_security_note(note);
    }

    for example in spec.examples {
        description = description.with_example(example);
    }

    description
}

fn description_spec(kind: ToolDescriptionKind) -> DescriptionSpec {
    match kind {
        ToolDescriptionKind::TerraformPlan => DescriptionSpec {
            summary: "Execute 'terraform plan' to show changes that would be made to infrastructure",
            usage_guide: "This tool generates an execution plan showing what Terraform will do when you apply \
                the configuration. It's a safe operation that doesn't make any changes to real infrastructure.",
            constraints: vec![
                "Terraform must be initialized in the target directory",
                "Valid Terraform configuration files must exist",
            ],
            error_hints: vec![
                (
                    "Init Required",
                    "Run terraform init first to initialize the working directory",
                ),
                (
                    "No Configuration",
                    "Ensure .tf files exist in the project directory",
                ),
            ],
            security_notes: vec![
                "This is a read-only operation - no infrastructure changes are made",
            ],
            examples: vec![ToolExample {
                title: "Basic Plan Generation".to_string(),
                description: "Generate a plan for the current Terraform configuration".to_string(),
                input: json!({}),
                expected_output:
                    "JSON-formatted plan showing resources to be created, modified, or destroyed"
                        .to_string(),
            }],
        },
        ToolDescriptionKind::TerraformApply => DescriptionSpec {
            summary: "Apply Terraform configuration to create, update, or delete infrastructure resources",
            usage_guide: "This tool executes the changes shown in a terraform plan. It will modify real infrastructure \
                according to your configuration. Always review the plan before applying changes.",
            constraints: vec![
                "TFMCP_ALLOW_DANGEROUS_OPS must be set to true",
                "Terraform must be initialized",
                "Valid configuration files must exist",
            ],
            error_hints: vec![
                (
                    "Permission Denied",
                    "Set TFMCP_ALLOW_DANGEROUS_OPS=true to enable apply operations",
                ),
                ("Init Required", "Run terraform init first"),
                (
                    "Auto-approve Blocked",
                    "Set TFMCP_ALLOW_AUTO_APPROVE=true for auto-approval",
                ),
            ],
            security_notes: vec![
                "This operation modifies real infrastructure - use with caution",
                "All apply operations are logged for audit purposes",
                "Production directory patterns are automatically blocked",
            ],
            examples: vec![
                ToolExample {
                    title: "Apply with Manual Approval".to_string(),
                    description: "Apply changes with interactive approval".to_string(),
                    input: json!({"auto_approve": false}),
                    expected_output: "Terraform apply output showing resources created/modified"
                        .to_string(),
                },
                ToolExample {
                    title: "Auto-approved Apply".to_string(),
                    description: "Apply changes automatically without manual confirmation"
                        .to_string(),
                    input: json!({"auto_approve": true}),
                    expected_output: "Terraform apply output with automatic approval".to_string(),
                },
            ],
        },
        ToolDescriptionKind::TerraformValidate => DescriptionSpec {
            summary: "Validate Terraform configuration files for syntax and semantic correctness",
            usage_guide: "This tool checks your Terraform configuration for syntax errors, missing required \
                arguments, and other validation issues. It's a safe operation that doesn't access remote state.",
            constraints: vec!["Terraform configuration files must exist in the directory"],
            error_hints: vec![
                (
                    "No Configuration",
                    "Ensure .tf files exist in the project directory",
                ),
                (
                    "Syntax Error",
                    "Check Terraform configuration syntax using terraform fmt",
                ),
            ],
            security_notes: vec!["This is a safe, local operation that doesn't modify anything"],
            examples: vec![ToolExample {
                title: "Basic Validation".to_string(),
                description: "Validate the current Terraform configuration".to_string(),
                input: json!({}),
                expected_output: "Validation result with success status and any error messages"
                    .to_string(),
            }],
        },
        ToolDescriptionKind::TerraformValidateDetailed => DescriptionSpec {
            summary: "Perform comprehensive validation with best practice analysis and detailed diagnostics",
            usage_guide: "This tool performs detailed validation including Terraform's built-in checks plus \
                additional best practice recommendations. It provides detailed diagnostics with file \
                locations and actionable suggestions.",
            constraints: vec!["Terraform configuration files must exist"],
            error_hints: vec![
                ("No Configuration", "Add .tf files to the project directory"),
                (
                    "Best Practice Violation",
                    "Review suggestions to improve configuration quality",
                ),
            ],
            security_notes: vec![
                "Includes security best practice checks",
                "No infrastructure access required - purely local analysis",
            ],
            examples: vec![ToolExample {
                title: "Comprehensive Analysis".to_string(),
                description: "Get detailed validation with best practices".to_string(),
                input: json!({}),
                expected_output:
                    "Detailed report with errors, warnings, suggestions, and file locations"
                        .to_string(),
            }],
        },
        ToolDescriptionKind::TerraformDestroy => DescriptionSpec {
            summary: "Destroy all resources defined in the Terraform configuration",
            usage_guide: "This tool destroys all infrastructure resources managed by Terraform in the current \
                configuration. This is a destructive operation that cannot be undone. Use with extreme caution.",
            constraints: vec![
                "TFMCP_ALLOW_DANGEROUS_OPS must be set to true",
                "Terraform must be initialized",
                "State file must exist with managed resources",
            ],
            error_hints: vec![
                (
                    "Permission Denied",
                    "Set TFMCP_ALLOW_DANGEROUS_OPS=true to enable destroy operations",
                ),
                ("No State", "No Terraform state found - nothing to destroy"),
                (
                    "Auto-approve Blocked",
                    "Set TFMCP_ALLOW_AUTO_APPROVE=true for auto-approval",
                ),
            ],
            security_notes: vec![
                "⚠️ DESTRUCTIVE OPERATION - destroys real infrastructure",
                "All destroy operations are logged for audit purposes",
                "Production directory patterns are automatically blocked",
                "Consider backing up important data before destruction",
            ],
            examples: vec![ToolExample {
                title: "Destroy with Confirmation".to_string(),
                description: "Destroy resources with manual confirmation".to_string(),
                input: json!({"auto_approve": false}),
                expected_output: "Terraform destroy output showing resources removed".to_string(),
            }],
        },
        ToolDescriptionKind::TerraformAnalyze => DescriptionSpec {
            summary: "Analyze Terraform configuration files to extract detailed information about resources, variables, and providers",
            usage_guide: "This tool parses your Terraform configuration files and provides comprehensive analysis \
                including resources, variables, outputs, and provider information. Useful for understanding \
                configuration structure and dependencies.",
            constraints: vec!["Terraform configuration files must exist in the directory"],
            error_hints: vec![
                (
                    "No Configuration",
                    "Add .tf files to the project directory to analyze",
                ),
                (
                    "Parse Error",
                    "Check Terraform syntax - configuration files may be malformed",
                ),
            ],
            security_notes: vec![
                "Analysis is performed locally without accessing remote resources",
            ],
            examples: vec![
                ToolExample {
                    title: "Analyze Current Configuration".to_string(),
                    description: "Analyze all .tf files in the current directory".to_string(),
                    input: json!({}),
                    expected_output:
                        "Structured analysis showing resources, variables, outputs, and providers"
                            .to_string(),
                },
                ToolExample {
                    title: "Analyze Specific Path".to_string(),
                    description: "Analyze configuration in a specific directory".to_string(),
                    input: json!({"path": "/path/to/terraform/config"}),
                    expected_output: "Analysis results for the specified directory".to_string(),
                },
            ],
        },
        ToolDescriptionKind::ListResources => DescriptionSpec {
            summary: "List all resources currently managed by Terraform in the state file",
            usage_guide: "This tool shows all infrastructure resources that are currently being managed by \
                Terraform according to the state file. Useful for understanding what resources exist \
                and their identifiers.",
            constraints: vec!["Terraform must be initialized", "State file must exist"],
            error_hints: vec![
                (
                    "No State",
                    "Run terraform apply to create managed resources first",
                ),
                (
                    "Init Required",
                    "Initialize Terraform working directory first",
                ),
            ],
            security_notes: vec!["Reads from local state file - no remote access required"],
            examples: vec![ToolExample {
                title: "List All Resources".to_string(),
                description: "Show all resources in the current state".to_string(),
                input: json!({}),
                expected_output: "Array of resource identifiers managed by Terraform".to_string(),
            }],
        },
        ToolDescriptionKind::GetState => DescriptionSpec {
            summary: "Retrieve the current Terraform state information",
            usage_guide: "This tool provides access to the current Terraform state, showing the real-world \
                resources that Terraform is managing and their current configuration.",
            constraints: vec!["Terraform must be initialized", "State file must exist"],
            error_hints: vec![
                (
                    "No State",
                    "No state file found - apply configuration first",
                ),
                (
                    "Corrupted State",
                    "State file may be corrupted - check terraform state list",
                ),
            ],
            security_notes: vec![
                "State may contain sensitive information",
                "Read-only operation - state is not modified",
            ],
            examples: vec![ToolExample {
                title: "Get Current State".to_string(),
                description: "Retrieve the complete Terraform state".to_string(),
                input: json!({}),
                expected_output: "Terraform state information including resource details"
                    .to_string(),
            }],
        },
        ToolDescriptionKind::InitTerraform => DescriptionSpec {
            summary: "Initialize a Terraform working directory and download required providers",
            usage_guide: "This tool initializes a Terraform working directory by downloading and installing \
                provider plugins, modules, and setting up the backend. This is typically the first \
                command to run in a new Terraform configuration.",
            constraints: vec![
                "Terraform configuration files must exist",
                "Network access required for downloading providers",
            ],
            error_hints: vec![
                (
                    "No Configuration",
                    "Create .tf files with provider configuration first",
                ),
                (
                    "Network Error",
                    "Check internet connectivity for provider downloads",
                ),
                (
                    "Backend Error",
                    "Verify backend configuration if using remote state",
                ),
            ],
            security_notes: vec![
                "Downloads providers from trusted Terraform Registry",
                "May create local state file containing infrastructure information",
            ],
            examples: vec![ToolExample {
                title: "Initialize Working Directory".to_string(),
                description: "Set up Terraform environment for the current configuration"
                    .to_string(),
                input: json!({}),
                expected_output: "Initialization results showing downloaded providers and modules"
                    .to_string(),
            }],
        },
        ToolDescriptionKind::SetDirectory => DescriptionSpec {
            summary: "Change the active Terraform project directory for subsequent operations",
            usage_guide: "This tool allows you to switch between different Terraform projects by changing \
                the working directory. All subsequent Terraform operations will use the new directory.",
            constraints: vec![
                "Target directory must exist or be creatable",
                "Directory path must be valid",
            ],
            error_hints: vec![
                (
                    "Invalid Path",
                    "Ensure the directory path exists and is accessible",
                ),
                (
                    "Permission Error",
                    "Check read/write permissions for the target directory",
                ),
            ],
            security_notes: vec![
                "Automatically creates sample project if no .tf files exist",
                "Directory changes are logged for audit purposes",
            ],
            examples: vec![ToolExample {
                title: "Switch to Project Directory".to_string(),
                description: "Change to a specific Terraform project".to_string(),
                input: json!({"directory": "/path/to/terraform/project"}),
                expected_output: "Confirmation of directory change with current path".to_string(),
            }],
        },
        ToolDescriptionKind::SecurityStatus => DescriptionSpec {
            summary: "Get current security policy configuration and operational permissions",
            usage_guide: "This tool provides information about the current security settings, including \
                which operations are allowed, audit logging status, and security policy configuration.",
            constraints: vec![],
            error_hints: vec![],
            security_notes: vec![
                "Shows current security policy without exposing sensitive data",
                "Helps understand why certain operations might be blocked",
            ],
            examples: vec![ToolExample {
                title: "Check Security Configuration".to_string(),
                description: "Review current security settings and permissions".to_string(),
                input: json!({}),
                expected_output: "Security policy details, permissions, and audit status"
                    .to_string(),
            }],
        },
    }
}

/// Get all improved tool descriptions
pub fn get_all_tool_descriptions() -> std::collections::HashMap<String, ToolDescription> {
    [
        (
            "get_terraform_plan",
            create_terraform_plan_description as fn() -> ToolDescription,
        ),
        ("apply_terraform", create_terraform_apply_description),
        ("validate_terraform", create_terraform_validate_description),
        (
            "validate_terraform_detailed",
            create_terraform_validate_detailed_description,
        ),
        ("destroy_terraform", create_terraform_destroy_description),
        ("analyze_terraform", create_terraform_analyze_description),
        (
            "list_terraform_resources",
            create_list_resources_description,
        ),
        ("get_terraform_state", create_get_state_description),
        ("init_terraform", create_init_terraform_description),
        ("set_terraform_directory", create_set_directory_description),
        ("get_security_status", create_security_status_description),
    ]
    .into_iter()
    .map(|(name, create)| (name.to_string(), create()))
    .collect()
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
            assert!(!desc.summary.is_empty(), "Tool {name} missing summary");
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
