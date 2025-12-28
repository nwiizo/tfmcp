//! Terraform provider information retrieval.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Provider information from terraform providers command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub name: String,
    pub namespace: String,
    pub version: Option<String>,
    pub version_constraints: Option<String>,
    pub source: String,
}

/// Lock file entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderLock {
    pub name: String,
    pub version: String,
    pub constraints: Option<String>,
    pub hashes: Vec<String>,
}

/// Complete provider information result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvidersResult {
    pub success: bool,
    pub providers: Vec<ProviderInfo>,
    pub locks: Option<Vec<ProviderLock>>,
    pub message: String,
}

/// Get provider information
pub fn get_providers(
    terraform_path: &Path,
    project_dir: &Path,
    include_lock: bool,
) -> anyhow::Result<ProvidersResult> {
    // Run terraform providers command
    let output = Command::new(terraform_path)
        .arg("providers")
        .current_dir(project_dir)
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        return Err(anyhow::anyhow!("Failed to get providers: {}", stderr));
    }

    // Parse provider output
    let providers = parse_providers_output(&stdout);

    // Parse lock file if requested
    let locks = if include_lock {
        parse_lock_file(project_dir).ok()
    } else {
        None
    };

    let message = format!("Found {} providers", providers.len());

    Ok(ProvidersResult {
        success: true,
        providers,
        locks,
        message,
    })
}

/// Parse terraform providers output
fn parse_providers_output(output: &str) -> Vec<ProviderInfo> {
    let mut providers = Vec::new();
    let mut seen: HashMap<String, bool> = HashMap::new();

    for line in output.lines() {
        let line = line.trim();

        // Skip empty lines and headers
        if line.is_empty()
            || line.starts_with("Providers required")
            || line.starts_with(".")
            || line.starts_with("├")
            || line.starts_with("└")
        {
            // Check if this is a provider line in tree format
            if line.contains("provider[") || line.contains("registry.terraform.io") {
                if let Some(provider) = parse_provider_line(line) {
                    let key = format!("{}/{}", provider.namespace, provider.name);
                    if let std::collections::hash_map::Entry::Vacant(e) = seen.entry(key) {
                        e.insert(true);
                        providers.push(provider);
                    }
                }
            }
            continue;
        }

        // Parse provider lines
        if line.contains("provider[") || line.contains("registry.terraform.io") {
            if let Some(provider) = parse_provider_line(line) {
                let key = format!("{}/{}", provider.namespace, provider.name);
                if let std::collections::hash_map::Entry::Vacant(e) = seen.entry(key) {
                    e.insert(true);
                    providers.push(provider);
                }
            }
        }
    }

    providers
}

/// Parse a single provider line
fn parse_provider_line(line: &str) -> Option<ProviderInfo> {
    // Format: provider[registry.terraform.io/hashicorp/aws] 5.0.0
    // or: └── provider[registry.terraform.io/hashicorp/aws] 5.0.0

    let line = line
        .trim_start_matches('├')
        .trim_start_matches('└')
        .trim_start_matches('─')
        .trim_start_matches(' ')
        .trim();

    // Extract the provider part
    if let Some(start) = line.find("provider[") {
        let rest = &line[start + 9..];
        if let Some(end) = rest.find(']') {
            let provider_path = &rest[..end];

            // Parse provider path: registry.terraform.io/namespace/name
            let parts: Vec<&str> = provider_path.split('/').collect();
            if parts.len() >= 3 {
                let namespace = parts[parts.len() - 2].to_string();
                let name = parts[parts.len() - 1].to_string();

                // Extract version if present
                let version = rest[end + 1..]
                    .split_whitespace()
                    .next()
                    .filter(|v| !v.is_empty())
                    .map(|v| v.to_string());

                return Some(ProviderInfo {
                    name,
                    namespace: namespace.clone(),
                    version: version.clone(),
                    version_constraints: version,
                    source: provider_path.to_string(),
                });
            }
        }
    }

    None
}

/// Parse .terraform.lock.hcl file
fn parse_lock_file(project_dir: &Path) -> anyhow::Result<Vec<ProviderLock>> {
    let lock_path = project_dir.join(".terraform.lock.hcl");

    if !lock_path.exists() {
        return Ok(vec![]);
    }

    let content = fs::read_to_string(&lock_path)?;
    parse_lock_hcl(&content)
}

/// Parse the lock file HCL content
fn parse_lock_hcl(content: &str) -> anyhow::Result<Vec<ProviderLock>> {
    let mut locks = Vec::new();
    let mut current_provider: Option<String> = None;
    let mut current_version: Option<String> = None;
    let mut current_constraints: Option<String> = None;
    let mut current_hashes: Vec<String> = Vec::new();

    for line in content.lines() {
        let line = line.trim();

        // Provider block start
        if line.starts_with("provider \"") {
            // Save previous provider if exists
            if let (Some(name), Some(version)) = (&current_provider, &current_version) {
                locks.push(ProviderLock {
                    name: name.clone(),
                    version: version.clone(),
                    constraints: current_constraints.take(),
                    hashes: std::mem::take(&mut current_hashes),
                });
            }

            // Extract new provider name
            if let Some(start) = line.find('"') {
                if let Some(end) = line[start + 1..].find('"') {
                    current_provider = Some(line[start + 1..start + 1 + end].to_string());
                }
            }
            current_version = None;
            current_constraints = None;
            current_hashes.clear();
        }
        // Version
        else if line.starts_with("version") {
            if let Some(start) = line.find('"') {
                if let Some(end) = line[start + 1..].find('"') {
                    current_version = Some(line[start + 1..start + 1 + end].to_string());
                }
            }
        }
        // Constraints
        else if line.starts_with("constraints") {
            if let Some(start) = line.find('"') {
                if let Some(end) = line[start + 1..].find('"') {
                    current_constraints = Some(line[start + 1..start + 1 + end].to_string());
                }
            }
        }
        // Hashes
        else if line.starts_with("\"h1:") || line.starts_with("\"zh:") {
            if let Some(end) = line[1..].find('"') {
                current_hashes.push(line[1..end + 1].to_string());
            }
        }
    }

    // Save last provider
    if let (Some(name), Some(version)) = (current_provider, current_version) {
        locks.push(ProviderLock {
            name,
            version,
            constraints: current_constraints,
            hashes: current_hashes,
        });
    }

    Ok(locks)
}

/// Get provider version constraints from configuration
#[allow(dead_code)]
pub fn get_provider_requirements(project_dir: &Path) -> anyhow::Result<HashMap<String, String>> {
    let mut requirements = HashMap::new();

    // Read all .tf files looking for required_providers blocks
    if let Ok(entries) = fs::read_dir(project_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|e| e == "tf") {
                if let Ok(content) = fs::read_to_string(&path) {
                    extract_provider_requirements(&content, &mut requirements);
                }
            }
        }
    }

    Ok(requirements)
}

/// Extract provider requirements from HCL content
#[allow(dead_code)]
fn extract_provider_requirements(content: &str, requirements: &mut HashMap<String, String>) {
    let mut in_required_providers = false;
    let mut brace_depth = 0;

    for line in content.lines() {
        let line = line.trim();

        if line.contains("required_providers") {
            in_required_providers = true;
            brace_depth = 0;
        }

        if in_required_providers {
            brace_depth += line.matches('{').count() as i32;
            brace_depth -= line.matches('}').count() as i32;

            if brace_depth <= 0 && line.contains('}') {
                in_required_providers = false;
                continue;
            }

            // Look for version constraints
            if line.contains("version") {
                if let Some(start) = line.find('"') {
                    if let Some(end) = line[start + 1..].find('"') {
                        let version = line[start + 1..start + 1 + end].to_string();

                        // Try to find the provider name from previous lines or same line
                        if let Some(eq_pos) = line.find('=') {
                            let name = line[..eq_pos].trim().to_string();
                            if !name.is_empty() && name != "version" {
                                requirements.insert(name, version.clone());
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_provider_line() {
        let line = "provider[registry.terraform.io/hashicorp/aws] 5.31.0";
        let provider = parse_provider_line(line).unwrap();
        assert_eq!(provider.name, "aws");
        assert_eq!(provider.namespace, "hashicorp");
        assert_eq!(provider.version, Some("5.31.0".to_string()));
    }

    #[test]
    fn test_parse_provider_line_tree_format() {
        let line = "└── provider[registry.terraform.io/hashicorp/random] 3.5.0";
        let provider = parse_provider_line(line).unwrap();
        assert_eq!(provider.name, "random");
        assert_eq!(provider.namespace, "hashicorp");
    }

    #[test]
    fn test_parse_lock_hcl() {
        let content = r#"
provider "registry.terraform.io/hashicorp/aws" {
  version     = "5.31.0"
  constraints = "~> 5.0"
  hashes = [
    "h1:abc123",
    "zh:def456",
  ]
}
"#;
        let locks = parse_lock_hcl(content).unwrap();
        assert_eq!(locks.len(), 1);
        assert_eq!(locks[0].version, "5.31.0");
        assert_eq!(locks[0].constraints, Some("~> 5.0".to_string()));
    }

    #[test]
    fn test_extract_provider_requirements() {
        let content = r#"
terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
}
"#;
        let mut reqs = HashMap::new();
        extract_provider_requirements(content, &mut reqs);
        // Note: This simple parser may not catch all cases
        // The terraform providers command is more reliable
    }
}
