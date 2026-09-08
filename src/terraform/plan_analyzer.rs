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
    /// Preserve Terraform's action ordering, including create-before-destroy.
    #[serde(default)]
    pub actions: Vec<String>,
    #[serde(default)]
    pub replace_paths: Vec<Vec<serde_json::Value>>,
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
    /// False for CLI event streams, which omit attribute-level changes.
    #[serde(default)]
    pub detailed: bool,
    #[serde(default)]
    pub output_changes: serde_json::Map<String, serde_json::Value>,
}

/// Terraform plan JSON output structure
#[derive(Debug, Deserialize)]
struct TerraformPlanJson {
    format_version: Option<String>,
    terraform_version: Option<String>,
    resource_changes: Option<Vec<PlanResourceChange>>,
    #[serde(skip)]
    detailed: bool,
    #[serde(default)]
    output_changes: HashMap<String, PlanChange>,
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
    before_sensitive: Option<serde_json::Value>,
    after_sensitive: Option<serde_json::Value>,
    #[serde(default)]
    replace_paths: Vec<Vec<serde_json::Value>>,
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
                actions: change
                    .change
                    .as_ref()
                    .map(|change| change.actions.clone())
                    .unwrap_or_default(),
                replace_paths: change
                    .change
                    .as_ref()
                    .map(|change| change.replace_paths.clone())
                    .unwrap_or_default(),
                before: change
                    .change
                    .as_ref()
                    .and_then(|c| redact_value(c.before.clone(), c.before_sensitive.as_ref())),
                after: change
                    .change
                    .as_ref()
                    .and_then(|c| redact_value(c.after.clone(), c.after_sensitive.as_ref())),
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
        detailed: plan.detailed,
        output_changes: plan
            .output_changes
            .into_iter()
            .map(|(name, change)| {
                (
                    name,
                    serde_json::json!({
                        "actions": change.actions,
                        "before": redact_value(change.before, change.before_sensitive.as_ref()),
                        "after": redact_value(change.after, change.after_sensitive.as_ref()),
                        "after_unknown": change.after_unknown,
                    }),
                )
            })
            .collect(),
    })
}

/// Parse terraform plan JSON (handles both single JSON and NDJSON format)
fn parse_plan_json(json_str: &str) -> anyhow::Result<TerraformPlanJson> {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str)
        && value.get("format_version").is_some()
    {
        anyhow::ensure!(
            value["format_version"]
                .as_str()
                .is_some_and(|version| version.starts_with("1.")),
            "Unsupported Terraform plan format version"
        );
        anyhow::ensure!(
            value.get("planned_values").is_some()
                || value.get("resource_changes").is_some()
                || value.get("output_changes").is_some(),
            "JSON does not contain a Terraform plan"
        );
        anyhow::ensure!(
            value["errored"] != true && value["complete"] != false,
            "Terraform plan is errored or incomplete"
        );
        let mut plan: TerraformPlanJson = serde_json::from_value(value)?;
        if let Some(changes) = &plan.resource_changes {
            anyhow::ensure!(
                changes.iter().all(|change| change
                    .change
                    .as_ref()
                    .is_some_and(|change| !change.actions.is_empty())),
                "Terraform plan contains a resource without change details"
            );
        }
        plan.detailed = true;
        return Ok(plan);
    }

    // If that fails, try to parse as NDJSON (newline-delimited JSON)
    // This is the format terraform plan -json outputs
    let mut resource_changes = Vec::new();
    let mut terraform_version = None;
    let mut format_version = None;
    let mut complete = false;

    for line in json_str.lines() {
        if line.trim().is_empty() {
            continue;
        }

        let obj: serde_json::Value = serde_json::from_str(line)
            .map_err(|_| anyhow::anyhow!("Invalid Terraform plan JSON event"))?;
        {
            // Check if this is a version message
            if let Some(v) = obj.get("terraform").and_then(|v| v.as_str()) {
                terraform_version = Some(v.to_string());
            }
            if let Some(v) = obj.get("ui").and_then(|v| v.as_str()) {
                format_version = Some(v.to_string());
            }

            match obj["type"].as_str() {
                Some("diagnostic") if obj["diagnostic"]["severity"] == "error" => {
                    anyhow::bail!("Terraform reported an error while planning");
                }
                Some("change_summary") if obj["changes"]["operation"] == "plan" => {
                    for key in ["add", "change", "remove"] {
                        anyhow::ensure!(
                            obj["changes"][key].as_u64().is_some(),
                            "Invalid Terraform plan change summary"
                        );
                    }
                    complete = true;
                }
                Some("planned_change") => {
                    resource_changes.push(parse_change_event(&obj["change"])?)
                }
                _ => {}
            }
        }
    }

    anyhow::ensure!(
        complete,
        "Terraform plan event stream is missing its completed plan summary"
    );
    Ok(TerraformPlanJson {
        format_version,
        terraform_version,
        resource_changes: if resource_changes.is_empty() {
            None
        } else {
            Some(resource_changes)
        },
        detailed: false,
        output_changes: HashMap::new(),
    })
}

fn parse_change_event(value: &serde_json::Value) -> anyhow::Result<PlanResourceChange> {
    let required = |value: &serde_json::Value| {
        value
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| anyhow::anyhow!("Incomplete Terraform planned_change event"))
    };
    let action = required(&value["action"])?;
    let actions = match action.as_str() {
        "replace" => vec!["delete".to_string(), "create".to_string()],
        "noop" => vec!["no-op".to_string()],
        "create" | "update" | "delete" | "read" | "move" | "import" => vec![action],
        _ => anyhow::bail!("Unsupported Terraform plan action"),
    };
    Ok(PlanResourceChange {
        address: required(&value["resource"]["addr"])?,
        resource_type: required(&value["resource"]["resource_type"])?,
        provider_name: value["resource"]["implied_provider"]
            .as_str()
            .map(str::to_owned),
        change: Some(PlanChange {
            actions,
            before: None,
            after: None,
            after_unknown: None,
            before_sensitive: None,
            after_sensitive: None,
            replace_paths: Vec::new(),
        }),
    })
}

impl PlanAnalysis {
    /// A redacted Terraform-compatible plan JSON view without variables, configuration, or prior state.
    pub fn to_plan_json(&self) -> anyhow::Result<String> {
        anyhow::ensure!(
            self.detailed,
            "A saved plan is required for attribute-level plan JSON"
        );
        let changes: Vec<_> = self
            .resource_changes
            .iter()
            .map(|change| {
                serde_json::json!({
                    "address": change.address,
                    "type": change.resource_type,
                    "provider_name": change.provider,
                    "change": {
                        "actions": change.actions,
                        "before": change.before,
                        "after": change.after,
                        "after_unknown": change.after_unknown,
                        "replace_paths": change.replace_paths,
                    }
                })
            })
            .collect();
        Ok(serde_json::to_string(&serde_json::json!({
            "format_version": self.format_version,
            "terraform_version": self.terraform_version,
            "resource_changes": changes,
            "output_changes": self.output_changes,
        }))?)
    }
}

/// Apply Terraform's sensitivity tree before values leave the server.
pub(super) fn redact_value(
    value: Option<serde_json::Value>,
    sensitive: Option<&serde_json::Value>,
) -> Option<serde_json::Value> {
    use serde_json::Value;
    let mut value = value?;
    match (sensitive, &mut value) {
        (Some(Value::Bool(true)), _) => value = Value::String("[sensitive]".to_string()),
        (Some(Value::Object(mask)), Value::Object(fields)) => {
            for (key, field) in fields {
                if let Some(redacted) = redact_value(Some(field.take()), mask.get(key)) {
                    *field = redacted;
                }
            }
        }
        (Some(Value::Array(mask)), Value::Array(fields)) => {
            for (index, field) in fields.iter_mut().enumerate() {
                if let Some(redacted) = redact_value(Some(field.take()), mask.get(index)) {
                    *field = redacted;
                }
            }
        }
        _ => {}
    }
    Some(value)
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
            serde_json::Value::String(s)
                if s.contains('.')
                    && !s.starts_with("http")
                    && !s.contains('/')
                    && s != current =>
            {
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
    fn real_plan_events_preserve_changes() -> anyhow::Result<()> {
        let output = concat!(
            "{\"type\":\"version\",\"terraform\":\"1.15.8\",\"ui\":\"1.2\"}\n",
            "{\"type\":\"planned_change\",\"change\":{\"resource\":{\"addr\":\"terraform_data.example\",\"resource_type\":\"terraform_data\",\"implied_provider\":\"terraform\"},\"action\":\"create\"}}\n",
            "{\"type\":\"change_summary\",\"changes\":{\"add\":1,\"change\":0,\"remove\":0,\"operation\":\"plan\"}}\n"
        );
        let analysis = analyze_plan(output, true)?;
        assert_eq!(analysis.summary.add, 1);
        assert_eq!(
            analysis.resource_changes[0].address,
            "terraform_data.example"
        );
        assert_eq!(analysis.terraform_version.as_deref(), Some("1.15.8"));
        Ok(())
    }

    #[test]
    fn invalid_or_incomplete_plan_is_not_an_empty_plan() {
        for output in [
            "",
            "not json",
            "{}",
            "{\"type\":\"version\",\"terraform\":\"1.15.8\",\"ui\":\"1.2\"}",
            "{\"format_version\":\"99.0\",\"planned_values\":{}}",
            "{\"type\":\"planned_change\",\"change\":{\"action\":\"create\"}}\n{\"type\":\"change_summary\",\"changes\":{\"add\":1,\"change\":0,\"remove\":0,\"operation\":\"plan\"}}",
        ] {
            assert!(analyze_plan(output, true).is_err(), "accepted: {output}");
        }
    }

    #[test]
    fn error_diagnostics_cannot_be_reviewed_as_successful_plan() {
        let output = concat!(
            "{\"type\":\"diagnostic\",\"diagnostic\":{\"severity\":\"error\",\"summary\":\"No value for required variable\"}}\n",
            "{\"type\":\"change_summary\",\"changes\":{\"add\":0,\"change\":0,\"remove\":0,\"operation\":\"plan\"}}"
        );
        assert!(analyze_plan(output, true).is_err());
    }

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
            actions: vec!["delete".to_string()],
            replace_paths: Vec::new(),
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
            actions: vec!["delete".to_string()],
            replace_paths: Vec::new(),
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
