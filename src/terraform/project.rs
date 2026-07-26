use crate::terraform::parser::TerraformParser;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

static MODULE_CALL_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"module\s+"([^"]+)""#).expect("Invalid module call regex"));
static BACKEND_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"backend\s+"([^"]+)""#).expect("Invalid backend regex"));
static REQUIRED_PROVIDERS_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"required_providers\s*\{"#).expect("Invalid required providers regex")
});

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerraformProjectInspection {
    pub root: String,
    pub total_tf_files: usize,
    pub total_directories: usize,
    pub entrypoints: Vec<TerraformEntrypoint>,
    pub modules: Vec<TerraformDirectorySummary>,
    pub directories: Vec<TerraformDirectorySummary>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerraformEntrypoint {
    pub path: String,
    pub relative_path: String,
    pub confidence: u8,
    pub reasons: Vec<String>,
    pub file_count: usize,
    pub resource_count: usize,
    pub variable_count: usize,
    pub output_count: usize,
    pub provider_names: Vec<String>,
    pub module_calls: Vec<String>,
    pub has_backend: bool,
    pub has_required_providers: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerraformDirectorySummary {
    pub path: String,
    pub relative_path: String,
    pub file_count: usize,
    pub resource_count: usize,
    pub variable_count: usize,
    pub output_count: usize,
    pub provider_names: Vec<String>,
    pub module_calls: Vec<String>,
    pub has_backend: bool,
    pub has_required_providers: bool,
    pub has_main_tf: bool,
    pub has_variables_tf: bool,
    pub has_outputs_tf: bool,
}

pub fn inspect_project(root: &Path) -> anyhow::Result<TerraformProjectInspection> {
    if !root.exists() {
        return Err(anyhow::anyhow!(
            "Project directory does not exist: {}",
            root.display()
        ));
    }

    let directories = scan_terraform_directories(root)?;
    let total_tf_files = directories.iter().map(|dir| dir.file_count).sum();
    let entrypoints = rank_entrypoints(&directories);
    let modules = directories
        .iter()
        .filter(|dir| is_module_path(&dir.relative_path))
        .cloned()
        .collect();

    let mut warnings = Vec::new();
    if total_tf_files == 0 {
        warnings.push("No Terraform files found".to_string());
    }
    if entrypoints.is_empty() && total_tf_files > 0 {
        warnings.push(
            "Terraform files exist, but no strong root module entrypoint was detected".to_string(),
        );
    }

    Ok(TerraformProjectInspection {
        root: root.to_string_lossy().to_string(),
        total_tf_files,
        total_directories: directories.len(),
        entrypoints,
        modules,
        directories,
        warnings,
    })
}

pub fn detect_entrypoints(root: &Path) -> anyhow::Result<Vec<TerraformEntrypoint>> {
    Ok(inspect_project(root)?.entrypoints)
}

fn scan_terraform_directories(root: &Path) -> anyhow::Result<Vec<TerraformDirectorySummary>> {
    let mut directories = Vec::new();
    collect_terraform_directories(root, root, &mut directories)?;
    directories.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    Ok(directories)
}

fn collect_terraform_directories(
    root: &Path,
    dir: &Path,
    directories: &mut Vec<TerraformDirectorySummary>,
) -> anyhow::Result<()> {
    let mut tf_files = Vec::new();
    let mut child_dirs = Vec::new();

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if should_skip_dir(&path) {
                continue;
            }
            child_dirs.push(path);
        } else if is_terraform_file(&path) {
            tf_files.push(path);
        }
    }

    if !tf_files.is_empty() {
        directories.push(summarize_directory(root, dir, &tf_files)?);
    }

    child_dirs.sort();
    for child in child_dirs {
        collect_terraform_directories(root, &child, directories)?;
    }

    Ok(())
}

fn summarize_directory(
    root: &Path,
    dir: &Path,
    tf_files: &[PathBuf],
) -> anyhow::Result<TerraformDirectorySummary> {
    let mut resource_count = 0;
    let mut variable_count = 0;
    let mut output_count = 0;
    let mut provider_names = BTreeSet::new();
    let mut module_calls = BTreeSet::new();
    let mut has_backend = false;
    let mut has_required_providers = false;
    let mut has_main_tf = false;
    let mut has_variables_tf = false;
    let mut has_outputs_tf = false;

    for file in tf_files {
        let file_name = file
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default();
        has_main_tf |= file_name == "main.tf";
        has_variables_tf |= file_name == "variables.tf";
        has_outputs_tf |= file_name == "outputs.tf";

        let content = std::fs::read_to_string(file)?;
        let parser = TerraformParser::new(content.clone());

        resource_count += parser.parse_resources(&file_name).len();
        variable_count += parser.parse_variables().len();
        output_count += parser.parse_outputs().len();
        for provider in parser.parse_providers() {
            provider_names.insert(provider.name);
        }
        for captures in MODULE_CALL_REGEX.captures_iter(&content) {
            module_calls.insert(captures[1].to_string());
        }
        has_backend |= BACKEND_REGEX.is_match(&content);
        has_required_providers |= REQUIRED_PROVIDERS_REGEX.is_match(&content);
    }

    Ok(TerraformDirectorySummary {
        path: dir.to_string_lossy().to_string(),
        relative_path: relative_path(root, dir),
        file_count: tf_files.len(),
        resource_count,
        variable_count,
        output_count,
        provider_names: provider_names.into_iter().collect(),
        module_calls: module_calls.into_iter().collect(),
        has_backend,
        has_required_providers,
        has_main_tf,
        has_variables_tf,
        has_outputs_tf,
    })
}

fn rank_entrypoints(directories: &[TerraformDirectorySummary]) -> Vec<TerraformEntrypoint> {
    let mut entrypoints: Vec<_> = directories
        .iter()
        .filter_map(|dir| {
            let (confidence, reasons) = entrypoint_confidence(dir);
            if confidence >= 20 {
                Some(TerraformEntrypoint {
                    path: dir.path.clone(),
                    relative_path: dir.relative_path.clone(),
                    confidence,
                    reasons,
                    file_count: dir.file_count,
                    resource_count: dir.resource_count,
                    variable_count: dir.variable_count,
                    output_count: dir.output_count,
                    provider_names: dir.provider_names.clone(),
                    module_calls: dir.module_calls.clone(),
                    has_backend: dir.has_backend,
                    has_required_providers: dir.has_required_providers,
                })
            } else {
                None
            }
        })
        .collect();

    entrypoints.sort_by(|a, b| {
        b.confidence
            .cmp(&a.confidence)
            .then_with(|| a.relative_path.cmp(&b.relative_path))
    });
    entrypoints
}

fn entrypoint_confidence(dir: &TerraformDirectorySummary) -> (u8, Vec<String>) {
    let mut score: i32 = 0;
    let mut reasons = Vec::new();

    if dir.relative_path == "." {
        score += 15;
        reasons.push("repository root contains Terraform files".to_string());
    }
    if dir.has_required_providers {
        score += 25;
        reasons.push("defines required_providers".to_string());
    }
    if dir.has_backend {
        score += 25;
        reasons.push("defines backend configuration".to_string());
    }
    if !dir.provider_names.is_empty() {
        score += 15;
        reasons.push("declares providers".to_string());
    }
    if dir.resource_count > 0 {
        score += 15;
        reasons.push("contains managed resources".to_string());
    }
    if !dir.module_calls.is_empty() {
        score += 10;
        reasons.push("calls child modules".to_string());
    }
    if dir.has_main_tf {
        score += 5;
        reasons.push("contains main.tf".to_string());
    }
    if is_module_path(&dir.relative_path) {
        score -= 35;
        reasons.push("appears to be a reusable child module".to_string());
    }

    (score.clamp(0, 100) as u8, reasons)
}

fn is_terraform_file(path: &Path) -> bool {
    path.is_file()
        && path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".tf") || name.ends_with(".tf.json"))
}

fn should_skip_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| matches!(name, ".git" | ".terraform" | ".terragrunt-cache"))
}

fn is_module_path(relative_path: &str) -> bool {
    relative_path == "modules"
        || relative_path.starts_with("modules/")
        || relative_path.starts_with(r"modules\")
}

fn relative_path(root: &Path, dir: &Path) -> String {
    dir.strip_prefix(root)
        .ok()
        .filter(|path| !path.as_os_str().is_empty())
        .map(|path| {
            path.components()
                .map(|component| component.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/")
        })
        .unwrap_or_else(|| ".".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn inspect_project_detects_root_and_child_module() {
        let temp = TempDir::new().unwrap();
        write_file(
            &temp.path().join("main.tf"),
            r#"
terraform {
  backend "local" {}
  required_providers {
    aws = {
      source = "hashicorp/aws"
    }
  }
}

provider "aws" {}
module "network" {
  source = "./modules/network"
}
"#,
        );
        let module_dir = temp.path().join("modules/network");
        std::fs::create_dir_all(&module_dir).unwrap();
        write_file(
            &module_dir.join("main.tf"),
            r#"
resource "aws_vpc" "main" {}
variable "cidr" {}
output "vpc_id" {}
"#,
        );

        let inspection = inspect_project(temp.path()).unwrap();

        assert_eq!(inspection.total_tf_files, 2);
        assert_eq!(inspection.modules.len(), 1);
        assert_eq!(inspection.entrypoints[0].relative_path, ".");
        assert!(inspection.entrypoints[0].confidence > 50);
        assert_eq!(inspection.entrypoints[0].module_calls, vec!["network"]);
    }

    #[test]
    fn inspect_project_ignores_terraform_working_directory() {
        let temp = TempDir::new().unwrap();
        let hidden = temp.path().join(".terraform/modules/cache");
        std::fs::create_dir_all(&hidden).unwrap();
        write_file(
            &hidden.join("main.tf"),
            r#"resource "null_resource" "ignored" {}"#,
        );

        let inspection = inspect_project(temp.path()).unwrap();

        assert_eq!(inspection.total_tf_files, 0);
        assert!(inspection.entrypoints.is_empty());
        assert_eq!(inspection.warnings, vec!["No Terraform files found"]);
    }

    #[test]
    fn module_paths_accept_platform_separators() {
        assert!(is_module_path("modules/network"));
        assert!(is_module_path(r"modules\network"));
        assert!(!is_module_path("examples/network"));
    }

    fn write_file(path: &Path, content: &str) {
        std::fs::write(path, content).unwrap();
    }
}
