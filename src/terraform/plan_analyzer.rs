//! Plan analyzer for detailed terraform plan analysis with risk scoring.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Risk level for plan changes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// A single resource change from terraform plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceChange {
    pub address: String,
    pub resource_type: String,
    pub provider: String,
    pub action: String,
    pub before: Option<serde_json::Value>,
    pub after: Option<serde_json::Value>,
    pub after_unknown: Option<serde_json::Value>,
}

/// Change summary statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChangeSummary {
    pub add: i32,
    pub change: i32,
    pub destroy: i32,
    pub replace: i32,
    pub no_op: i32,
}

/// Risk assessment for the plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub level: RiskLevel,
    pub score: i32,
    pub warnings: Vec<String>,
    pub recommendations: Vec<String>,
}

/// Dependency impact analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyImpact {
    pub resource: String,
    pub affected_by: Vec<String>,
    pub affects: Vec<String>,
}

/// Complete plan analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanAnalysis {
    pub summary: ChangeSummary,
    pub resource_changes: Vec<ResourceChange>,
    pub risk_assessment: RiskAssessment,
    pub dependency_impacts: Vec<DependencyImpact>,
    pub terraform_version: Option<String>,
    pub format_version: Option<String>,
}

/// Terraform plan JSON output structure
#[derive(Debug, Deserialize)]
struct TerraformPlanJson {
    format_version: Option<String>,
    terraform_version: Option<String>,
    resource_changes: Option<Vec<PlanResourceChange>>,
    #[allow(dead_code)]
    prior_state: Option<serde_json::Value>,
    #[allow(dead_code)]
    configuration: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct PlanResourceChange {
    address: String,
    #[serde(rename = "type")]
    resource_type: String,
    provider_name: Option<String>,
    change: Option<PlanChange>,
}

#[derive(Debug, Deserialize)]
struct PlanChange {
    actions: Vec<String>,
    before: Option<serde_json::Value>,
    after: Option<serde_json::Value>,
    after_unknown: Option<serde_json::Value>,
}

/// High-risk resource types that require extra caution
const HIGH_RISK_RESOURCES: &[&str] = &[
    "aws_db_instance",
    "aws_rds_cluster",
    "aws_elasticache_cluster",
    "aws_elasticsearch_domain",
    "aws_opensearch_domain",
    "google_sql_database_instance",
    "azurerm_sql_database",
    "azurerm_postgresql_server",
    "aws_s3_bucket",
    "google_storage_bucket",
    "azurerm_storage_account",
    "aws_iam_role",
    "aws_iam_policy",
    "google_project_iam_binding",
    "azurerm_role_assignment",
    "aws_security_group",
    "google_compute_firewall",
    "azurerm_network_security_group",
    "aws_vpc",
    "google_compute_network",
    "azurerm_virtual_network",
    "aws_kms_key",
    "google_kms_crypto_key",
    "azurerm_key_vault",
];

/// Analyze terraform plan JSON output
pub fn analyze_plan(plan_json: &str, include_risk: bool) -> anyhow::Result<PlanAnalysis> {
    // Try to parse as JSON array of lines (terraform plan -json outputs NDJSON)
    let plan = parse_plan_json(plan_json)?;

    let mut summary = ChangeSummary::default();
    let mut resource_changes = Vec::new();

    if let Some(changes) = plan.resource_changes {
        for change in changes {
            let action = if let Some(ref c) = change.change {
                actions_to_string(&c.actions)
            } else {
                "unknown".to_string()
            };

            // Update summary
            match action.as_str() {
                "create" => summary.add += 1,
                "update" => summary.change += 1,
                "delete" => summary.destroy += 1,
                "replace" | "create_delete" | "delete_create" => summary.replace += 1,
                "no-op" | "read" => summary.no_op += 1,
                _ => {}
            }

            let rc = ResourceChange {
                address: change.address.clone(),
                resource_type: change.resource_type.clone(),
                provider: change
                    .provider_name
                    .unwrap_or_else(|| "unknown".to_string()),
                action: action.clone(),
                before: change.change.as_ref().and_then(|c| c.before.clone()),
                after: change.change.as_ref().and_then(|c| c.after.clone()),
                after_unknown: change.change.as_ref().and_then(|c| c.after_unknown.clone()),
            };
            resource_changes.push(rc);
        }
    }

    let risk_assessment = if include_risk {
        assess_risk(&resource_changes, &summary)
    } else {
        RiskAssessment {
            level: RiskLevel::Low,
            score: 0,
            warnings: vec![],
            recommendations: vec![],
        }
    };

    let dependency_impacts = analyze_dependencies(&resource_changes);

    Ok(PlanAnalysis {
        summary,
        resource_changes,
        risk_assessment,
        dependency_impacts,
        terraform_version: plan.terraform_version,
        format_version: plan.format_version,
    })
}

/// Parse terraform plan JSON (handles both single JSON and NDJSON format)
fn parse_plan_json(json_str: &str) -> anyhow::Result<TerraformPlanJson> {
    // First try to parse as a single JSON object
    if let Ok(plan) = serde_json::from_str::<TerraformPlanJson>(json_str) {
        return Ok(plan);
    }

    // If that fails, try to parse as NDJSON (newline-delimited JSON)
    // This is the format terraform plan -json outputs
    let mut resource_changes = Vec::new();
    let mut terraform_version = None;
    let mut format_version = None;

    for line in json_str.lines() {
        if line.trim().is_empty() {
            continue;
        }

        if let Ok(obj) = serde_json::from_str::<serde_json::Value>(line) {
            // Check if this is a version message
            if let Some(v) = obj.get("terraform_version").and_then(|v| v.as_str()) {
                terraform_version = Some(v.to_string());
            }
            if let Some(v) = obj.get("format_version").and_then(|v| v.as_str()) {
                format_version = Some(v.to_string());
            }

            // Check if this is a resource_drift or planned_change message
            if let Some(change_type) = obj.get("type").and_then(|t| t.as_str()) {
                if change_type == "planned_change" || change_type == "resource_drift" {
                    if let Some(change) = obj.get("change") {
                        if let Ok(rc) = serde_json::from_value::<PlanResourceChange>(change.clone())
                        {
                            resource_changes.push(rc);
                        }
                    }
                }
            }
        }
    }

    Ok(TerraformPlanJson {
        format_version,
        terraform_version,
        resource_changes: if resource_changes.is_empty() {
            None
        } else {
            Some(resource_changes)
        },
        prior_state: None,
        configuration: None,
    })
}

/// Convert action array to a single action string
fn actions_to_string(actions: &[String]) -> String {
    match actions.len() {
        0 => "no-op".to_string(),
        1 => actions[0].clone(),
        2 => {
            if actions.contains(&"create".to_string()) && actions.contains(&"delete".to_string()) {
                "replace".to_string()
            } else {
                actions.join("_")
            }
        }
        _ => actions.join("_"),
    }
}

/// Assess risk based on resource changes
fn assess_risk(changes: &[ResourceChange], summary: &ChangeSummary) -> RiskAssessment {
    let mut score = 0;
    let mut warnings = Vec::new();
    let mut recommendations = Vec::new();

    // Base score from change counts
    score += summary.destroy * 30;
    score += summary.replace * 20;
    score += summary.change * 5;
    score += summary.add * 2;

    // Check for high-risk resources
    for change in changes {
        let is_high_risk = HIGH_RISK_RESOURCES
            .iter()
            .any(|&r| change.resource_type == r);

        if is_high_risk {
            match change.action.as_str() {
                "delete" => {
                    score += 50;
                    warnings.push(format!(
                        "CRITICAL: High-risk resource '{}' will be DESTROYED",
                        change.address
                    ));
                }
                "replace" | "create_delete" | "delete_create" => {
                    score += 40;
                    warnings.push(format!(
                        "WARNING: High-risk resource '{}' will be REPLACED (data loss possible)",
                        change.address
                    ));
                }
                "update" => {
                    score += 15;
                    warnings.push(format!(
                        "CAUTION: High-risk resource '{}' will be modified",
                        change.address
                    ));
                }
                _ => {}
            }
        }

        // Check for IAM/security changes
        let is_security_resource = change.resource_type.contains("iam")
            || change.resource_type.contains("security")
            || change.resource_type.contains("firewall");
        if is_security_resource && change.action != "no-op" && change.action != "read" {
            score += 10;
            warnings.push(format!(
                "Security-related resource '{}' will be modified",
                change.address
            ));
        }

        // Check for network changes
        let is_network_resource = change.resource_type.contains("vpc")
            || change.resource_type.contains("network")
            || change.resource_type.contains("subnet");
        if is_network_resource && (change.action == "delete" || change.action.contains("replace")) {
            score += 25;
            warnings.push(format!(
                "Network infrastructure '{}' change may cause connectivity issues",
                change.address
            ));
        }
    }

    // Generate recommendations
    if summary.destroy > 0 {
        recommendations.push("Review all resources marked for destruction carefully".to_string());
        recommendations.push("Ensure backups exist for any stateful resources".to_string());
    }

    if summary.replace > 0 {
        recommendations
            .push("Resources being replaced may have brief downtime or data loss".to_string());
    }

    if score > 50 {
        recommendations.push("Consider applying changes during a maintenance window".to_string());
        recommendations.push("Have a rollback plan ready".to_string());
    }

    let level = match score {
        0..=10 => RiskLevel::Low,
        11..=30 => RiskLevel::Medium,
        31..=60 => RiskLevel::High,
        _ => RiskLevel::Critical,
    };

    RiskAssessment {
        level,
        score,
        warnings,
        recommendations,
    }
}

/// Analyze dependencies between resources
fn analyze_dependencies(changes: &[ResourceChange]) -> Vec<DependencyImpact> {
    let mut impacts = Vec::new();
    let mut resource_refs: HashMap<String, Vec<String>> = HashMap::new();

    // Build a map of references from after values
    for change in changes {
        if let Some(after) = &change.after {
            let refs = extract_references(after, &change.address);
            for ref_addr in refs {
                resource_refs
                    .entry(ref_addr)
                    .or_default()
                    .push(change.address.clone());
            }
        }
    }

    // Create impact analysis for each changed resource
    for change in changes {
        if change.action == "no-op" || change.action == "read" {
            continue;
        }

        let affected_by: Vec<String> = changes
            .iter()
            .filter(|c| {
                c.address != change.address
                    && (c.action == "delete"
                        || c.action.contains("replace")
                        || c.action == "update")
            })
            .filter(|c| {
                // Check if this change might affect the current resource
                if let Some(after) = &change.after {
                    let refs = extract_references(after, &change.address);
                    refs.contains(&c.address)
                } else {
                    false
                }
            })
            .map(|c| c.address.clone())
            .collect();

        let affects = resource_refs
            .get(&change.address)
            .cloned()
            .unwrap_or_default();

        if !affected_by.is_empty() || !affects.is_empty() {
            impacts.push(DependencyImpact {
                resource: change.address.clone(),
                affected_by,
                affects,
            });
        }
    }

    impacts
}

/// Extract resource references from a JSON value
fn extract_references(value: &serde_json::Value, current_addr: &str) -> Vec<String> {
    let mut refs = Vec::new();

    fn walk(v: &serde_json::Value, refs: &mut Vec<String>, current: &str) {
        match v {
            serde_json::Value::String(s) => {
                // Look for resource address patterns like "aws_instance.example"
                if s.contains('.') && !s.starts_with("http") && !s.contains('/') && s != current {
                    // Check if it looks like a resource address
                    let parts: Vec<&str> = s.split('.').collect();
                    if parts.len() >= 2
                        && !parts[0].is_empty()
                        && parts[0]
                            .chars()
                            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
                    {
                        refs.push(s.clone());
                    }
                }
            }
            serde_json::Value::Array(arr) => {
                for item in arr {
                    walk(item, refs, current);
                }
            }
            serde_json::Value::Object(obj) => {
                for (_, v) in obj {
                    walk(v, refs, current);
                }
            }
            _ => {}
        }
    }

    walk(value, &mut refs, current_addr);
    refs.sort();
    refs.dedup();
    refs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_actions_to_string() {
        assert_eq!(actions_to_string(&[]), "no-op");
        assert_eq!(actions_to_string(&["create".to_string()]), "create");
        assert_eq!(
            actions_to_string(&["create".to_string(), "delete".to_string()]),
            "replace"
        );
    }

    #[test]
    fn test_risk_assessment_empty() {
        let changes = vec![];
        let summary = ChangeSummary::default();
        let risk = assess_risk(&changes, &summary);
        assert_eq!(risk.level, RiskLevel::Low);
        assert_eq!(risk.score, 0);
    }

    #[test]
    fn test_risk_assessment_destroy() {
        let changes = vec![ResourceChange {
            address: "aws_instance.example".to_string(),
            resource_type: "aws_instance".to_string(),
            provider: "aws".to_string(),
            action: "delete".to_string(),
            before: None,
            after: None,
            after_unknown: None,
        }];
        let summary = ChangeSummary {
            destroy: 1,
            ..Default::default()
        };
        let risk = assess_risk(&changes, &summary);
        assert!(risk.score > 0);
    }

    #[test]
    fn test_high_risk_resource() {
        let changes = vec![ResourceChange {
            address: "aws_db_instance.main".to_string(),
            resource_type: "aws_db_instance".to_string(),
            provider: "aws".to_string(),
            action: "delete".to_string(),
            before: None,
            after: None,
            after_unknown: None,
        }];
        let summary = ChangeSummary {
            destroy: 1,
            ..Default::default()
        };
        let risk = assess_risk(&changes, &summary);
        assert_eq!(risk.level, RiskLevel::Critical);
        assert!(risk.warnings.iter().any(|w| w.contains("CRITICAL")));
    }
}
