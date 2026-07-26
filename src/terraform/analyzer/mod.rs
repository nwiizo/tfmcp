//! Terraform module analyzer façade.
//!
//! Public functions are re-exported from focused implementation modules to keep
//! the service API stable while limiting structural coupling in each analyzer area.

pub mod graph;
pub mod guidelines;
pub mod health;
pub mod refactoring;

pub use graph::build_dependency_graph;
pub use guidelines::check_guidelines;
pub use health::analyze_module_health;
pub use refactoring::suggest_refactoring;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terraform::analyzer::health::{
        analyze_cohesion, calculate_metrics, get_resource_category,
    };
    use crate::terraform::model::TerraformAnalysis;
    use crate::terraform::model::core::{TerraformOutput, TerraformProvider, TerraformVariable};
    use std::collections::HashMap;

    #[test]
    fn test_check_guidelines_missing_type() {
        let analysis = TerraformAnalysis {
            project_directory: "/test".to_string(),
            file_count: 1,
            resources: vec![],
            variables: vec![
                TerraformVariable {
                    name: "var_with_type".to_string(),
                    description: Some("Has type".to_string()),
                    type_: Some("string".to_string()),
                    default: None,
                },
                TerraformVariable {
                    name: "var_without_type".to_string(),
                    description: Some("No type".to_string()),
                    type_: None,
                    default: None,
                },
            ],
            outputs: vec![],
            providers: vec![],
        };
        let file_contents = HashMap::new();
        let result = check_guidelines(&analysis, &file_contents);

        assert_eq!(result.variables_missing_type.len(), 1);
        assert_eq!(result.variables_missing_type[0], "var_without_type");
    }

    #[test]
    fn test_check_guidelines_missing_description() {
        let analysis = TerraformAnalysis {
            project_directory: "/test".to_string(),
            file_count: 1,
            resources: vec![],
            variables: vec![
                TerraformVariable {
                    name: "var_with_desc".to_string(),
                    description: Some("Has description".to_string()),
                    type_: Some("string".to_string()),
                    default: None,
                },
                TerraformVariable {
                    name: "var_without_desc".to_string(),
                    description: None,
                    type_: Some("string".to_string()),
                    default: None,
                },
            ],
            outputs: vec![
                TerraformOutput {
                    name: "output_with_desc".to_string(),
                    description: Some("Has description".to_string()),
                    value: None,
                },
                TerraformOutput {
                    name: "output_without_desc".to_string(),
                    description: None,
                    value: None,
                },
            ],
            providers: vec![],
        };
        let file_contents = HashMap::new();
        let result = check_guidelines(&analysis, &file_contents);

        assert_eq!(result.variables_missing_description.len(), 1);
        assert_eq!(result.variables_missing_description[0], "var_without_desc");
        assert_eq!(result.outputs_missing_description.len(), 1);
        assert_eq!(result.outputs_missing_description[0], "output_without_desc");
    }

    #[test]
    fn test_check_guidelines_provider_version() {
        let analysis = TerraformAnalysis {
            project_directory: "/test".to_string(),
            file_count: 1,
            resources: vec![],
            variables: vec![],
            outputs: vec![],
            providers: vec![
                TerraformProvider {
                    name: "aws".to_string(),
                    version: Some("~> 5.0".to_string()),
                },
                TerraformProvider {
                    name: "random".to_string(),
                    version: None,
                },
            ],
        };
        let file_contents = HashMap::new();
        let result = check_guidelines(&analysis, &file_contents);

        assert_eq!(result.providers_missing_version.len(), 1);
        assert_eq!(result.providers_missing_version[0], "random");
    }

    #[test]
    fn test_compliance_score_calculation() {
        let analysis = TerraformAnalysis {
            project_directory: "/test".to_string(),
            file_count: 1,
            resources: vec![],
            variables: vec![TerraformVariable {
                name: "good_var".to_string(),
                description: Some("Good variable".to_string()),
                type_: Some("string".to_string()),
                default: None,
            }],
            outputs: vec![TerraformOutput {
                name: "good_output".to_string(),
                description: Some("Good output".to_string()),
                value: None,
            }],
            providers: vec![TerraformProvider {
                name: "aws".to_string(),
                version: Some("~> 5.0".to_string()),
            }],
        };
        let file_contents = HashMap::new();
        let result = check_guidelines(&analysis, &file_contents);

        // Perfect compliance should give 100 or close to it
        assert!(result.compliance_score >= 90);
    }

    fn create_test_analysis() -> TerraformAnalysis {
        TerraformAnalysis {
            project_directory: "/test/module".to_string(),
            file_count: 3,
            resources: vec![
                crate::terraform::model::core::TerraformResource {
                    resource_type: "aws_vpc".to_string(),
                    name: "main".to_string(),
                    file: "vpc.tf".to_string(),
                    provider: "aws".to_string(),
                },
                crate::terraform::model::core::TerraformResource {
                    resource_type: "aws_subnet".to_string(),
                    name: "public".to_string(),
                    file: "vpc.tf".to_string(),
                    provider: "aws".to_string(),
                },
                crate::terraform::model::core::TerraformResource {
                    resource_type: "aws_instance".to_string(),
                    name: "web".to_string(),
                    file: "compute.tf".to_string(),
                    provider: "aws".to_string(),
                },
            ],
            variables: vec![
                TerraformVariable {
                    name: "vpc_cidr".to_string(),
                    description: Some("VPC CIDR block".to_string()),
                    type_: Some("string".to_string()),
                    default: None,
                },
                TerraformVariable {
                    name: "instance_type".to_string(),
                    description: None,
                    type_: Some("string".to_string()),
                    default: Some(serde_json::json!("t3.micro")),
                },
            ],
            outputs: vec![TerraformOutput {
                name: "vpc_id".to_string(),
                description: Some("The VPC ID".to_string()),
                value: None,
            }],
            providers: vec![TerraformProvider {
                name: "aws".to_string(),
                version: Some("~> 5.0".to_string()),
            }],
        }
    }

    #[test]
    fn test_resource_category() {
        assert_eq!(get_resource_category("aws_vpc"), "networking-core");
        assert_eq!(get_resource_category("aws_subnet"), "networking-core");
        assert_eq!(get_resource_category("aws_instance"), "compute");
        assert_eq!(get_resource_category("aws_s3_bucket"), "storage");
        assert_eq!(
            get_resource_category("aws_security_group"),
            "networking-security"
        );
        assert_eq!(
            get_resource_category("aws_vpn_gateway"),
            "networking-connectivity"
        );
    }

    #[test]
    fn test_calculate_metrics() {
        let analysis = create_test_analysis();
        let mut file_contents = HashMap::new();
        file_contents.insert(
            "vpc.tf".to_string(),
            r#"
            resource "aws_vpc" "main" {}
            resource "aws_subnet" "public" {}
            "#
            .to_string(),
        );

        let metrics = calculate_metrics(&analysis, &file_contents);
        assert_eq!(metrics.variable_count, 2);
        assert_eq!(metrics.resource_count, 3);
        assert_eq!(metrics.output_count, 1);
    }

    #[test]
    fn test_analyze_cohesion() {
        let analysis = create_test_analysis();
        let cohesion = analyze_cohesion(&analysis);

        // Should have moderate cohesion (networking + compute)
        assert!(cohesion.score > 0);
        assert!(!cohesion.resource_type_groups.is_empty());
    }

    #[test]
    fn test_health_score_bounds() {
        let analysis = create_test_analysis();
        let file_contents = HashMap::new();
        let health = analyze_module_health(&analysis, &file_contents);

        assert!(health.health_score <= 100);
    }
}
