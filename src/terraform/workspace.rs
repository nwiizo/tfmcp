//! Terraform workspace management operations.

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

/// Workspace action to perform
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WorkspaceAction {
    List,
    Show,
    New,
    Select,
    Delete,
}

impl std::str::FromStr for WorkspaceAction {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "list" => Ok(WorkspaceAction::List),
            "show" => Ok(WorkspaceAction::Show),
            "new" | "create" => Ok(WorkspaceAction::New),
            "select" | "switch" => Ok(WorkspaceAction::Select),
            "delete" | "remove" => Ok(WorkspaceAction::Delete),
            _ => Err(anyhow::anyhow!(
                "Unknown workspace action: {s}. Valid actions: list, show, new, select, delete"
            )),
        }
    }
}

/// Workspace information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub name: String,
    pub current: bool,
}

/// Result of workspace operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceResult {
    pub success: bool,
    pub action: String,
    pub current_workspace: Option<String>,
    pub workspaces: Option<Vec<WorkspaceInfo>>,
    pub message: String,
}

/// Execute a workspace operation
pub fn execute_workspace(
    terraform_path: &Path,
    project_dir: &Path,
    action: WorkspaceAction,
    workspace_name: Option<&str>,
) -> anyhow::Result<WorkspaceResult> {
    match action {
        WorkspaceAction::List => list_workspaces(terraform_path, project_dir),
        WorkspaceAction::Show => show_workspace(terraform_path, project_dir),
        WorkspaceAction::New => {
            let name = workspace_name
                .ok_or_else(|| anyhow::anyhow!("Workspace name required for 'new' action"))?;
            run_named_workspace_action(terraform_path, project_dir, name, NamedWorkspaceAction::New)
        }
        WorkspaceAction::Select => {
            let name = workspace_name
                .ok_or_else(|| anyhow::anyhow!("Workspace name required for 'select' action"))?;
            run_named_workspace_action(
                terraform_path,
                project_dir,
                name,
                NamedWorkspaceAction::Select,
            )
        }
        WorkspaceAction::Delete => {
            let name = workspace_name
                .ok_or_else(|| anyhow::anyhow!("Workspace name required for 'delete' action"))?;
            delete_workspace(terraform_path, project_dir, name)
        }
    }
}

/// List all workspaces
fn list_workspaces(terraform_path: &Path, project_dir: &Path) -> anyhow::Result<WorkspaceResult> {
    let output = Command::new(terraform_path)
        .arg("workspace")
        .arg("list")
        .current_dir(project_dir)
        .output()?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "Failed to list workspaces: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut workspaces = Vec::new();
    let mut current_workspace = None;

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(name) = line.strip_prefix("* ") {
            current_workspace = Some(name.to_string());
            workspaces.push(WorkspaceInfo {
                name: name.to_string(),
                current: true,
            });
        } else {
            workspaces.push(WorkspaceInfo {
                name: line.to_string(),
                current: false,
            });
        }
    }

    let message = format!("Found {} workspaces", workspaces.len());
    Ok(WorkspaceResult {
        success: true,
        action: "list".to_string(),
        current_workspace,
        workspaces: Some(workspaces),
        message,
    })
}

/// Show current workspace
fn show_workspace(terraform_path: &Path, project_dir: &Path) -> anyhow::Result<WorkspaceResult> {
    let output = Command::new(terraform_path)
        .arg("workspace")
        .arg("show")
        .current_dir(project_dir)
        .output()?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "Failed to show workspace: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let current = String::from_utf8_lossy(&output.stdout).trim().to_string();

    Ok(WorkspaceResult {
        success: true,
        action: "show".to_string(),
        current_workspace: Some(current.clone()),
        workspaces: None,
        message: format!("Current workspace: {current}"),
    })
}

enum NamedWorkspaceAction {
    New,
    Select,
}

impl NamedWorkspaceAction {
    fn terraform_action(&self) -> &'static str {
        match self {
            Self::New => "new",
            Self::Select => "select",
        }
    }

    fn result_action(&self) -> &'static str {
        match self {
            Self::New => "new",
            Self::Select => "select",
        }
    }

    fn message(&self, name: &str) -> String {
        match self {
            Self::New => format!("Created and switched to workspace '{name}'"),
            Self::Select => format!("Switched to workspace '{name}'"),
        }
    }

    fn failure_prefix(&self) -> &'static str {
        match self {
            Self::New => "Failed to create workspace",
            Self::Select => "Failed to select workspace",
        }
    }

    fn mapped_error(&self, stderr: &str, name: &str) -> Option<anyhow::Error> {
        match self {
            Self::New if stderr.contains("already exists") => {
                Some(anyhow::anyhow!("Workspace '{name}' already exists"))
            }
            Self::Select
                if stderr.contains("doesn't exist") || stderr.contains("does not exist") =>
            {
                Some(anyhow::anyhow!("Workspace '{name}' does not exist"))
            }
            _ => None,
        }
    }

    fn validate(&self, name: &str) -> anyhow::Result<()> {
        if matches!(self, Self::New) && !is_valid_workspace_name(name) {
            return Err(anyhow::anyhow!(
                "Invalid workspace name: '{name}'. Names must be alphanumeric with hyphens or underscores"
            ));
        }
        Ok(())
    }
}

fn run_named_workspace_action(
    terraform_path: &Path,
    project_dir: &Path,
    name: &str,
    action: NamedWorkspaceAction,
) -> anyhow::Result<WorkspaceResult> {
    action.validate(name)?;
    let output = Command::new(terraform_path)
        .arg("workspace")
        .arg(action.terraform_action())
        .arg(name)
        .current_dir(project_dir)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if let Some(error) = action.mapped_error(&stderr, name) {
            return Err(error);
        }
        return Err(anyhow::anyhow!("{}: {}", action.failure_prefix(), stderr));
    }

    Ok(WorkspaceResult {
        success: true,
        action: action.result_action().to_string(),
        current_workspace: Some(name.to_string()),
        workspaces: None,
        message: action.message(name),
    })
}

/// Delete a workspace
fn delete_workspace(
    terraform_path: &Path,
    project_dir: &Path,
    name: &str,
) -> anyhow::Result<WorkspaceResult> {
    // Cannot delete the default workspace
    if name == "default" {
        return Err(anyhow::anyhow!("Cannot delete the 'default' workspace"));
    }

    // Check if trying to delete current workspace
    let current = show_workspace(terraform_path, project_dir)?;
    if current.current_workspace.as_deref() == Some(name) {
        return Err(anyhow::anyhow!(
            "Cannot delete workspace '{name}' because it is currently selected. Switch to another workspace first."
        ));
    }

    let output = Command::new(terraform_path)
        .arg("workspace")
        .arg("delete")
        .arg(name)
        .current_dir(project_dir)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("doesn't exist") || stderr.contains("does not exist") {
            return Err(anyhow::anyhow!("Workspace '{name}' does not exist"));
        }
        if stderr.contains("is not empty") {
            return Err(anyhow::anyhow!(
                "Workspace '{name}' is not empty. Use 'terraform workspace delete -force {name}' to force deletion."
            ));
        }
        return Err(anyhow::anyhow!("Failed to delete workspace: {stderr}"));
    }

    Ok(WorkspaceResult {
        success: true,
        action: "delete".to_string(),
        current_workspace: current.current_workspace,
        workspaces: None,
        message: format!("Deleted workspace '{name}'"),
    })
}

/// Validate workspace name
fn is_valid_workspace_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 100 {
        return false;
    }

    name.chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_action() {
        assert_eq!(
            "list".parse::<WorkspaceAction>().unwrap(),
            WorkspaceAction::List
        );
        assert_eq!(
            "show".parse::<WorkspaceAction>().unwrap(),
            WorkspaceAction::Show
        );
        assert_eq!(
            "new".parse::<WorkspaceAction>().unwrap(),
            WorkspaceAction::New
        );
        assert_eq!(
            "create".parse::<WorkspaceAction>().unwrap(),
            WorkspaceAction::New
        );
        assert_eq!(
            "select".parse::<WorkspaceAction>().unwrap(),
            WorkspaceAction::Select
        );
        assert_eq!(
            "delete".parse::<WorkspaceAction>().unwrap(),
            WorkspaceAction::Delete
        );
    }

    #[test]
    fn test_valid_workspace_name() {
        assert!(is_valid_workspace_name("dev"));
        assert!(is_valid_workspace_name("prod-us-east-1"));
        assert!(is_valid_workspace_name("staging_v2"));
        assert!(!is_valid_workspace_name(""));
        assert!(!is_valid_workspace_name("name with spaces"));
        assert!(!is_valid_workspace_name("name/slash"));
    }
}
