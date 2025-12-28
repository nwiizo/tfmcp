//! Terraform refresh operations.
//!
//! Note: `terraform refresh` is deprecated. The recommended approach is to use
//! `terraform apply -refresh-only` which is what this module implements internally.

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

/// Result of refresh operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshResult {
    pub success: bool,
    pub resources_updated: i32,
    pub output: String,
    pub changes: Vec<RefreshChange>,
    pub message: String,
}

/// A single refresh change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshChange {
    pub resource_address: String,
    pub change_type: RefreshChangeType,
    pub detail: Option<String>,
}

/// Type of refresh change
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RefreshChangeType {
    Updated,
    Drifted,
    Unchanged,
}

/// Execute terraform refresh (via apply -refresh-only)
pub fn execute_refresh(
    terraform_path: &Path,
    project_dir: &Path,
    target: Option<&str>,
) -> anyhow::Result<RefreshResult> {
    let mut cmd = Command::new(terraform_path);
    cmd.arg("apply")
        .arg("-refresh-only")
        .arg("-auto-approve")
        .arg("-json");

    // If targeting a specific resource
    if let Some(target_addr) = target {
        cmd.arg(format!("-target={}", target_addr));
    }

    let output = cmd.current_dir(project_dir).output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        return Err(anyhow::anyhow!("Refresh failed: {}", stderr));
    }

    // Parse JSON output to extract changes
    let changes = parse_refresh_output(&stdout);
    let resources_updated = changes
        .iter()
        .filter(|c| c.change_type == RefreshChangeType::Updated)
        .count() as i32;

    let message = if resources_updated > 0 {
        format!("Refreshed {} resources", resources_updated)
    } else {
        "No resources needed refreshing".to_string()
    };

    Ok(RefreshResult {
        success: true,
        resources_updated,
        output: stdout.to_string(),
        changes,
        message,
    })
}

/// Execute refresh with plan preview (no auto-approve)
#[allow(dead_code)]
pub fn preview_refresh(
    terraform_path: &Path,
    project_dir: &Path,
    target: Option<&str>,
) -> anyhow::Result<RefreshResult> {
    let mut cmd = Command::new(terraform_path);
    cmd.arg("plan").arg("-refresh-only").arg("-json");

    // If targeting a specific resource
    if let Some(target_addr) = target {
        cmd.arg(format!("-target={}", target_addr));
    }

    let output = cmd.current_dir(project_dir).output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        return Err(anyhow::anyhow!("Refresh preview failed: {}", stderr));
    }

    // Parse JSON output to extract changes
    let changes = parse_refresh_output(&stdout);
    let resources_updated = changes
        .iter()
        .filter(|c| c.change_type == RefreshChangeType::Drifted)
        .count() as i32;

    let message = if resources_updated > 0 {
        format!(
            "{} resources have drifted and would be updated",
            resources_updated
        )
    } else {
        "No drift detected - state is up to date".to_string()
    };

    Ok(RefreshResult {
        success: true,
        resources_updated,
        output: stdout.to_string(),
        changes,
        message,
    })
}

/// Parse refresh output to extract changes
fn parse_refresh_output(json_output: &str) -> Vec<RefreshChange> {
    let mut changes = Vec::new();

    for line in json_output.lines() {
        if line.trim().is_empty() {
            continue;
        }

        if let Ok(obj) = serde_json::from_str::<serde_json::Value>(line) {
            // Check for resource_drift or planned_change messages
            if let Some(msg_type) = obj.get("type").and_then(|t| t.as_str()) {
                match msg_type {
                    "resource_drift" => {
                        if let Some(change) = obj.get("change") {
                            if let Some(resource) = change.get("resource") {
                                if let Some(addr) = resource.get("addr").and_then(|a| a.as_str()) {
                                    changes.push(RefreshChange {
                                        resource_address: addr.to_string(),
                                        change_type: RefreshChangeType::Drifted,
                                        detail: Some("Resource has drifted from state".to_string()),
                                    });
                                }
                            }
                        }
                    }
                    "planned_change" => {
                        if let Some(change) = obj.get("change") {
                            if let Some(resource) = change.get("resource") {
                                if let Some(addr) = resource.get("addr").and_then(|a| a.as_str()) {
                                    let action = change
                                        .get("action")
                                        .and_then(|a| a.as_str())
                                        .unwrap_or("update");

                                    if action == "update" {
                                        changes.push(RefreshChange {
                                            resource_address: addr.to_string(),
                                            change_type: RefreshChangeType::Updated,
                                            detail: Some("State will be updated".to_string()),
                                        });
                                    }
                                }
                            }
                        }
                    }
                    "apply_complete" | "change_summary" => {
                        // Skip summary messages
                    }
                    _ => {}
                }
            }
        }
    }

    changes
}

/// Get a list of resources that might need refreshing
#[allow(dead_code)]
pub fn get_stale_resources(
    terraform_path: &Path,
    project_dir: &Path,
) -> anyhow::Result<Vec<String>> {
    // Run a refresh-only plan to detect drift
    let result = preview_refresh(terraform_path, project_dir, None)?;

    let stale: Vec<String> = result
        .changes
        .into_iter()
        .filter(|c| c.change_type == RefreshChangeType::Drifted)
        .map(|c| c.resource_address)
        .collect();

    Ok(stale)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_output() {
        let changes = parse_refresh_output("");
        assert!(changes.is_empty());
    }

    #[test]
    fn test_parse_drift_message() {
        let json =
            r#"{"type":"resource_drift","change":{"resource":{"addr":"aws_instance.example"}}}"#;
        let changes = parse_refresh_output(json);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].resource_address, "aws_instance.example");
        assert_eq!(changes[0].change_type, RefreshChangeType::Drifted);
    }

    #[test]
    fn test_parse_planned_change() {
        let json = r#"{"type":"planned_change","change":{"resource":{"addr":"aws_s3_bucket.data"},"action":"update"}}"#;
        let changes = parse_refresh_output(json);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].change_type, RefreshChangeType::Updated);
    }
}
