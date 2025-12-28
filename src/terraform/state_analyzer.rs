//! State analyzer for Terraform state analysis with drift detection.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Resource statistics grouped by provider
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderStats {
    pub name: String,
    pub resource_count: i32,
    pub resource_types: Vec<String>,
}

/// Resource statistics grouped by type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeStats {
    pub resource_type: String,
    pub count: i32,
    pub addresses: Vec<String>,
}

/// Drift detection result for a single resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftResult {
    pub address: String,
    pub resource_type: String,
    pub drift_type: DriftType,
    pub details: Option<String>,
}

/// Type of drift detected
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DriftType {
    /// Resource exists in state but may be modified in cloud
    Modified,
    /// Resource exists in state but not in cloud (deleted externally)
    Deleted,
    /// Resource exists in cloud but not in state (created externally)
    Orphaned,
    /// Resource configuration differs from state
    ConfigurationDrift,
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub name: String,
    pub status: HealthStatus,
    pub message: String,
}

/// Health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
}

/// Resource in state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateResource {
    pub address: String,
    pub resource_type: String,
    pub provider: String,
    pub module: Option<String>,
    pub tainted: bool,
    pub attributes: Option<serde_json::Value>,
}

/// Complete state analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateAnalysis {
    pub total_resources: i32,
    pub providers: Vec<ProviderStats>,
    pub types: Vec<TypeStats>,
    pub resources: Vec<StateResource>,
    pub drift_results: Vec<DriftResult>,
    pub health_checks: Vec<HealthCheck>,
    pub state_version: Option<i32>,
    pub terraform_version: Option<String>,
    pub serial: Option<i64>,
}

/// Terraform state JSON structure
#[derive(Debug, Deserialize)]
struct TerraformStateJson {
    version: Option<i32>,
    terraform_version: Option<String>,
    serial: Option<i64>,
    resources: Option<Vec<StateResourceJson>>,
}

#[derive(Debug, Deserialize)]
struct StateResourceJson {
    #[serde(rename = "type")]
    resource_type: String,
    name: String,
    provider: String,
    module: Option<String>,
    instances: Option<Vec<StateInstance>>,
}

#[derive(Debug, Deserialize)]
struct StateInstance {
    attributes: Option<serde_json::Value>,
    #[serde(default)]
    status: Option<String>,
    index_key: Option<serde_json::Value>,
}

/// Analyze terraform state
pub fn analyze_state(
    state_json: &str,
    resource_type_filter: Option<&str>,
    detect_drift: bool,
) -> anyhow::Result<StateAnalysis> {
    let state: TerraformStateJson = serde_json::from_str(state_json)?;

    let mut resources = Vec::new();
    let mut provider_map: HashMap<String, ProviderStats> = HashMap::new();
    let mut type_map: HashMap<String, TypeStats> = HashMap::new();

    if let Some(state_resources) = state.resources {
        for resource in state_resources {
            // Apply type filter if specified
            if let Some(filter) = resource_type_filter {
                if !resource.resource_type.contains(filter) {
                    continue;
                }
            }

            let provider_name = extract_provider_name(&resource.provider);

            // Process each instance of the resource
            if let Some(instances) = resource.instances {
                for (idx, instance) in instances.iter().enumerate() {
                    let address = if instances.len() == 1 {
                        format_address(&resource.module, &resource.resource_type, &resource.name)
                    } else if let Some(ref key) = instance.index_key {
                        format!(
                            "{}[{}]",
                            format_address(
                                &resource.module,
                                &resource.resource_type,
                                &resource.name
                            ),
                            key
                        )
                    } else {
                        format!(
                            "{}[{}]",
                            format_address(
                                &resource.module,
                                &resource.resource_type,
                                &resource.name
                            ),
                            idx
                        )
                    };

                    let tainted = instance.status.as_ref().is_some_and(|s| s == "tainted");

                    resources.push(StateResource {
                        address: address.clone(),
                        resource_type: resource.resource_type.clone(),
                        provider: provider_name.clone(),
                        module: resource.module.clone(),
                        tainted,
                        attributes: instance.attributes.clone(),
                    });

                    // Update provider stats
                    let provider_stats =
                        provider_map
                            .entry(provider_name.clone())
                            .or_insert_with(|| ProviderStats {
                                name: provider_name.clone(),
                                resource_count: 0,
                                resource_types: Vec::new(),
                            });
                    provider_stats.resource_count += 1;
                    if !provider_stats
                        .resource_types
                        .contains(&resource.resource_type)
                    {
                        provider_stats
                            .resource_types
                            .push(resource.resource_type.clone());
                    }

                    // Update type stats
                    let type_stats = type_map
                        .entry(resource.resource_type.clone())
                        .or_insert_with(|| TypeStats {
                            resource_type: resource.resource_type.clone(),
                            count: 0,
                            addresses: Vec::new(),
                        });
                    type_stats.count += 1;
                    type_stats.addresses.push(address);
                }
            }
        }
    }

    let drift_results = if detect_drift {
        detect_drift_issues(&resources)
    } else {
        Vec::new()
    };

    let health_checks = run_health_checks(&resources, &provider_map);

    let mut providers: Vec<ProviderStats> = provider_map.into_values().collect();
    providers.sort_by(|a, b| b.resource_count.cmp(&a.resource_count));

    let mut types: Vec<TypeStats> = type_map.into_values().collect();
    types.sort_by(|a, b| b.count.cmp(&a.count));

    Ok(StateAnalysis {
        total_resources: resources.len() as i32,
        providers,
        types,
        resources,
        drift_results,
        health_checks,
        state_version: state.version,
        terraform_version: state.terraform_version,
        serial: state.serial,
    })
}

/// Format resource address
fn format_address(module: &Option<String>, resource_type: &str, name: &str) -> String {
    match module {
        Some(m) if !m.is_empty() => format!("{}.{}.{}", m, resource_type, name),
        _ => format!("{}.{}", resource_type, name),
    }
}

/// Extract provider name from provider string (e.g., "provider[\"registry.terraform.io/hashicorp/aws\"]")
fn extract_provider_name(provider: &str) -> String {
    if provider.contains('/') {
        // Extract from full provider path
        provider
            .rsplit('/')
            .next()
            .unwrap_or(provider)
            .trim_end_matches(']')
            .trim_end_matches('"')
            .to_string()
    } else {
        provider.to_string()
    }
}

/// Detect potential drift issues (heuristic-based)
fn detect_drift_issues(resources: &[StateResource]) -> Vec<DriftResult> {
    let mut drift_results = Vec::new();

    for resource in resources {
        // Check for tainted resources
        if resource.tainted {
            drift_results.push(DriftResult {
                address: resource.address.clone(),
                resource_type: resource.resource_type.clone(),
                drift_type: DriftType::Modified,
                details: Some("Resource is marked as tainted".to_string()),
            });
        }

        // Check for resources without attributes (may indicate issues)
        if resource.attributes.is_none() {
            drift_results.push(DriftResult {
                address: resource.address.clone(),
                resource_type: resource.resource_type.clone(),
                drift_type: DriftType::ConfigurationDrift,
                details: Some("Resource has no attributes in state".to_string()),
            });
        }

        // Check for data sources that might have stale data
        if resource.resource_type.starts_with("data.") {
            drift_results.push(DriftResult {
                address: resource.address.clone(),
                resource_type: resource.resource_type.clone(),
                drift_type: DriftType::ConfigurationDrift,
                details: Some(
                    "Data source may have stale data - run refresh to update".to_string(),
                ),
            });
        }
    }

    drift_results
}

/// Run health checks on the state
fn run_health_checks(
    resources: &[StateResource],
    providers: &HashMap<String, ProviderStats>,
) -> Vec<HealthCheck> {
    let mut checks = Vec::new();

    // Check for empty state
    if resources.is_empty() {
        checks.push(HealthCheck {
            name: "state_not_empty".to_string(),
            status: HealthStatus::Warning,
            message: "State is empty - no resources are being managed".to_string(),
        });
    } else {
        checks.push(HealthCheck {
            name: "state_not_empty".to_string(),
            status: HealthStatus::Healthy,
            message: format!("State contains {} resources", resources.len()),
        });
    }

    // Check for tainted resources
    let tainted_count = resources.iter().filter(|r| r.tainted).count();
    if tainted_count > 0 {
        checks.push(HealthCheck {
            name: "no_tainted_resources".to_string(),
            status: HealthStatus::Warning,
            message: format!(
                "{} tainted resources found - they will be recreated on next apply",
                tainted_count
            ),
        });
    } else {
        checks.push(HealthCheck {
            name: "no_tainted_resources".to_string(),
            status: HealthStatus::Healthy,
            message: "No tainted resources found".to_string(),
        });
    }

    // Check for resources in modules
    let module_resources = resources.iter().filter(|r| r.module.is_some()).count();
    if module_resources > 0 && resources.len() > 10 {
        let percentage = (module_resources as f64 / resources.len() as f64) * 100.0;
        if percentage < 50.0 {
            checks.push(HealthCheck {
                name: "module_usage".to_string(),
                status: HealthStatus::Warning,
                message: format!(
                    "Only {:.0}% of resources are in modules - consider modularizing",
                    percentage
                ),
            });
        } else {
            checks.push(HealthCheck {
                name: "module_usage".to_string(),
                status: HealthStatus::Healthy,
                message: format!("{:.0}% of resources are organized in modules", percentage),
            });
        }
    }

    // Check for provider diversity (potential complexity)
    if providers.len() > 5 {
        checks.push(HealthCheck {
            name: "provider_count".to_string(),
            status: HealthStatus::Warning,
            message: format!(
                "{} providers in use - consider if all are necessary",
                providers.len()
            ),
        });
    } else {
        checks.push(HealthCheck {
            name: "provider_count".to_string(),
            status: HealthStatus::Healthy,
            message: format!("{} providers in use", providers.len()),
        });
    }

    checks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_provider_name() {
        assert_eq!(
            extract_provider_name("provider[\"registry.terraform.io/hashicorp/aws\"]"),
            "aws"
        );
        assert_eq!(extract_provider_name("aws"), "aws");
    }

    #[test]
    fn test_format_address() {
        assert_eq!(
            format_address(&None, "aws_instance", "example"),
            "aws_instance.example"
        );
        assert_eq!(
            format_address(&Some("module.vpc".to_string()), "aws_subnet", "public"),
            "module.vpc.aws_subnet.public"
        );
    }

    #[test]
    fn test_analyze_empty_state() {
        let state_json = r#"{"version": 4, "terraform_version": "1.5.0", "resources": []}"#;
        let result = analyze_state(state_json, None, false).unwrap();
        assert_eq!(result.total_resources, 0);
        assert_eq!(result.state_version, Some(4));
    }

    #[test]
    fn test_analyze_state_with_resources() {
        let state_json = r#"{
            "version": 4,
            "terraform_version": "1.5.0",
            "serial": 123,
            "resources": [
                {
                    "type": "aws_instance",
                    "name": "example",
                    "provider": "provider[\"registry.terraform.io/hashicorp/aws\"]",
                    "instances": [
                        {"attributes": {"id": "i-12345"}}
                    ]
                }
            ]
        }"#;
        let result = analyze_state(state_json, None, false).unwrap();
        assert_eq!(result.total_resources, 1);
        assert_eq!(result.resources[0].address, "aws_instance.example");
        assert_eq!(result.resources[0].provider, "aws");
    }

    #[test]
    fn test_type_filter() {
        let state_json = r#"{
            "version": 4,
            "resources": [
                {
                    "type": "aws_instance",
                    "name": "web",
                    "provider": "aws",
                    "instances": [{"attributes": {}}]
                },
                {
                    "type": "aws_s3_bucket",
                    "name": "storage",
                    "provider": "aws",
                    "instances": [{"attributes": {}}]
                }
            ]
        }"#;
        let result = analyze_state(state_json, Some("s3"), false).unwrap();
        assert_eq!(result.total_resources, 1);
        assert!(result.resources[0].resource_type.contains("s3"));
    }
}
