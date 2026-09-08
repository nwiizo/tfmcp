//! Inspect the actual CLI target without creating infrastructure or changing state.
use super::execution;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionTarget {
    pub project_directory: String,
    pub workspace: String,
    pub terraform_version: String,
    pub backend: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TargetSnapshot {
    pub target: ExecutionTarget,
    backend_hash: Vec<u8>,
    lockfile_hash: Vec<u8>,
    data_directory: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPreflight {
    pub project_directory: String,
    pub terraform_version: Option<String>,
    pub workspace: Option<String>,
    pub backend: Option<String>,
    pub configuration_valid: Option<bool>,
    pub provider_lockfile_required: Option<bool>,
    pub state_status: String,
    /// Input variables and provider credentials are checked by the actual plan.
    pub input_values_checked: bool,
    pub ready: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Deserialize)]
struct ProviderSchemas {
    format_version: String,
    // Terraform omits this field when the configuration uses no providers.
    #[serde(default)]
    provider_schemas: std::collections::HashMap<String, serde_json::Value>,
}

pub(super) fn fingerprint(bytes: &[u8]) -> Vec<u8> {
    ring::digest::digest(&ring::digest::SHA256, bytes)
        .as_ref()
        .to_vec()
}

fn optional_file(path: &Path) -> Result<Vec<u8>> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(error.into()),
    }
}

pub(super) async fn target_snapshot(executable: &Path, directory: &Path) -> Result<TargetSnapshot> {
    let directory = directory
        .canonicalize()
        .context("Terraform project directory is unavailable")?;
    let version = execution::run(executable, &directory, &["version", "-json"]).await?;
    execution::ensure_success(&version, "version")?;
    let version: serde_json::Value = serde_json::from_slice(&version.stdout)?;
    let workspace = execution::run(executable, &directory, &["workspace", "show"]).await?;
    execution::ensure_success(&workspace, "workspace show")?;
    let workspace = String::from_utf8(workspace.stdout)?.trim().to_string();
    anyhow::ensure!(
        !workspace.is_empty(),
        "Terraform did not identify the selected workspace"
    );
    let data_directory = std::env::var_os("TF_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".terraform"));
    let data_directory = directory.join(data_directory);
    let backend_bytes = optional_file(&data_directory.join("terraform.tfstate"))?;
    let backend = if backend_bytes.is_empty() {
        "local".to_string()
    } else {
        let state: serde_json::Value = serde_json::from_slice(&backend_bytes)
            .context("Cannot read initialized backend metadata")?;
        state["backend"]["type"]
            .as_str()
            .unwrap_or("unknown")
            .to_string()
    };
    Ok(TargetSnapshot {
        target: ExecutionTarget {
            project_directory: directory.display().to_string(),
            workspace,
            terraform_version: version["terraform_version"]
                .as_str()
                .context("Terraform version is unavailable")?
                .to_string(),
            backend,
        },
        backend_hash: fingerprint(&backend_bytes),
        lockfile_hash: fingerprint(&optional_file(&directory.join(".terraform.lock.hcl"))?),
        data_directory,
    })
}

pub(super) async fn inspect(executable: &Path, directory: &Path) -> Result<ExecutionPreflight> {
    let mut result = ExecutionPreflight {
        project_directory: directory.display().to_string(),
        terraform_version: None,
        workspace: None,
        backend: None,
        configuration_valid: None,
        provider_lockfile_required: None,
        state_status: "unavailable".to_string(),
        input_values_checked: false,
        ready: false,
        blockers: Vec::new(),
        warnings: vec![
            "Required input values and provider credentials are checked when generating the plan"
                .to_string(),
        ],
    };
    let has_configuration = std::fs::read_dir(directory)?.try_fold(false, |found, entry| {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        Ok::<_, std::io::Error>(
            found
                || entry.path().is_file() && (name.ends_with(".tf") || name.ends_with(".tf.json")),
        )
    })?;
    if !has_configuration {
        result.blockers.push(
            "No Terraform configuration files were found in the selected directory".to_string(),
        );
    }
    match target_snapshot(executable, directory).await {
        Ok(snapshot) => {
            result.project_directory = snapshot.target.project_directory;
            result.terraform_version = Some(snapshot.target.terraform_version);
            result.workspace = Some(snapshot.target.workspace);
            result.backend = Some(snapshot.target.backend);
        }
        Err(error) => result.blockers.push(error.to_string()),
    }
    let validation = execution::run(executable, directory, &["validate", "-json"]).await?;
    match serde_json::from_slice::<serde_json::Value>(&validation.stdout) {
        Ok(value) if value["valid"].is_boolean() => {
            result.configuration_valid = value["valid"].as_bool();
            if result.configuration_valid != Some(true) || !validation.status.success() {
                result.blockers.extend(execution::diagnostics(&validation));
            }
        }
        _ => result
            .blockers
            .push("Terraform validation could not be read".to_string()),
    }
    if result.configuration_valid == Some(true) {
        let providers =
            execution::run(executable, directory, &["providers", "schema", "-json"]).await?;
        let schema = serde_json::from_slice::<ProviderSchemas>(&providers.stdout).ok();
        if providers.status.success()
            && let Some(schema) = schema
            && schema.format_version.starts_with("1.")
        {
            let required = schema
                .provider_schemas
                .keys()
                .any(|name| !name.starts_with("terraform.io/builtin/"));
            result.provider_lockfile_required = Some(required);
            if required && !directory.join(".terraform.lock.hcl").is_file() {
                result.blockers.push(
                    "Provider lockfile .terraform.lock.hcl is missing; run terraform init"
                        .to_string(),
                );
            }
        } else {
            result.blockers.push(
                "Could not verify the required provider schemas; run terraform init".to_string(),
            );
        }
    }
    let state = execution::run(executable, directory, &["state", "pull"]).await?;
    let stderr = String::from_utf8_lossy(&state.stderr);
    if state.status.success() {
        if state.stdout.is_empty() {
            result.state_status = "absent".to_string();
        } else if serde_json::from_slice::<serde_json::Value>(&state.stdout).is_ok() {
            result.state_status = "readable".to_string();
        } else {
            result
                .blockers
                .push("Terraform state is not valid JSON".to_string());
        }
    } else if stderr.contains("No state file") || stderr.contains("No state found") {
        result.state_status = "absent".to_string();
    } else {
        result.blockers.extend(execution::diagnostics(&state));
    }
    result.ready = result.blockers.is_empty() && result.configuration_valid == Some(true);
    Ok(result)
}
