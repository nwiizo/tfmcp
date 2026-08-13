//! Terraform taint/untaint operations.
//!
//! Note: terraform taint and untaint are deprecated in Terraform 1.5+.
//! The recommended approach is to use `terraform apply -replace=ADDRESS`.

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

/// Taint action type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TaintAction {
    Taint,
    Untaint,
}

impl std::str::FromStr for TaintAction {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "taint" => Ok(TaintAction::Taint),
            "untaint" => Ok(TaintAction::Untaint),
            _ => Err(anyhow::anyhow!(
                "Unknown taint action: {s}. Valid actions: taint, untaint"
            )),
        }
    }
}

/// Result of taint/untaint operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaintResult {
    pub success: bool,
    pub action: String,
    pub resource_address: String,
    pub message: String,
    pub deprecation_warning: Option<String>,
}

/// Execute taint or untaint operation
pub fn execute_taint(
    terraform_path: &Path,
    project_dir: &Path,
    action: TaintAction,
    address: &str,
) -> anyhow::Result<TaintResult> {
    // Check terraform version for deprecation warning
    let deprecation_warning = check_terraform_version_for_deprecation(terraform_path);

    let action_str = match action {
        TaintAction::Taint => "taint",
        TaintAction::Untaint => "untaint",
    };

    let output = Command::new(terraform_path)
        .arg(action_str)
        .arg(address)
        .current_dir(project_dir)
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if output.status.success() {
        let message = match action {
            TaintAction::Taint => {
                format!("Resource '{address}' has been marked as tainted")
            }
            TaintAction::Untaint => {
                format!("Resource '{address}' has been unmarked as tainted")
            }
        };

        Ok(TaintResult {
            success: true,
            action: action_str.to_string(),
            resource_address: address.to_string(),
            message,
            deprecation_warning,
        })
    } else {
        // Parse common error messages
        let message = if stderr.contains("No such resource instance") {
            format!("Resource '{address}' not found in state")
        } else if stderr.contains("not currently tainted") {
            format!("Resource '{address}' is not currently tainted")
        } else if stderr.contains("already tainted") {
            format!("Resource '{address}' is already tainted")
        } else if stdout.contains("Terraform has been successfully initialized") {
            // Sometimes terraform needs init first
            "Terraform needs to be initialized. Run 'terraform init' first.".to_string()
        } else {
            format!(
                "Failed to {} resource: {}",
                action_str,
                stderr.trim().replace('\n', " ")
            )
        };

        Ok(TaintResult {
            success: false,
            action: action_str.to_string(),
            resource_address: address.to_string(),
            message,
            deprecation_warning,
        })
    }
}

/// Check if terraform version is 1.5+ and return deprecation warning
fn check_terraform_version_for_deprecation(terraform_path: &Path) -> Option<String> {
    let output = Command::new(terraform_path)
        .arg("version")
        .arg("-json")
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout)
        && let Some(version_str) = json.get("terraform_version").and_then(|v| v.as_str())
        && taint_is_deprecated(version_str)
    {
        return Some(format!(
            "Note: 'terraform taint' and 'terraform untaint' are deprecated in Terraform {}. \
                    Consider using 'terraform apply -replace={}' instead for new workflows.",
            version_str, "RESOURCE_ADDRESS"
        ));
    }

    None
}

fn taint_is_deprecated(version: &str) -> bool {
    let mut parts = version.split('.');
    let major = parts.next().and_then(|part| part.parse::<i32>().ok());
    let minor = parts.next().and_then(|part| part.parse::<i32>().ok());

    matches!((major, minor), (Some(major), Some(minor)) if (major, minor) >= (1, 5))
}

/// Get the recommended replacement command for Terraform 1.5+
#[allow(dead_code)]
pub fn get_replacement_command(address: &str) -> String {
    format!("terraform apply -replace='{address}'")
}

/// Alternative: Plan with replace
#[allow(dead_code)]
pub fn plan_with_replace(
    terraform_path: &Path,
    project_dir: &Path,
    address: &str,
) -> anyhow::Result<String> {
    let output = Command::new(terraform_path)
        .arg("plan")
        .arg(format!("-replace={address}"))
        .current_dir(project_dir)
        .output()?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(anyhow::anyhow!(
            "Failed to plan with replace: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_action() {
        assert_eq!("taint".parse::<TaintAction>().unwrap(), TaintAction::Taint);
        assert_eq!(
            "untaint".parse::<TaintAction>().unwrap(),
            TaintAction::Untaint
        );
        assert!("invalid".parse::<TaintAction>().is_err());
    }

    #[test]
    fn test_replacement_command() {
        let cmd = get_replacement_command("aws_instance.example");
        assert!(cmd.contains("-replace="));
        assert!(cmd.contains("aws_instance.example"));
    }

    #[test]
    fn test_taint_deprecation_version_boundary() {
        assert!(!taint_is_deprecated("1.4.9"));
        assert!(taint_is_deprecated("1.5.0"));
        assert!(taint_is_deprecated("2.0.0"));
        assert!(!taint_is_deprecated("invalid"));
    }
}
