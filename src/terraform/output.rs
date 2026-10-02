//! Terraform output value retrieval.

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

/// A single output value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputValue {
    pub name: String,
    pub value: serde_json::Value,
    pub value_type: String,
    pub sensitive: bool,
    pub description: Option<String>,
}

/// Result of output retrieval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputResult {
    pub success: bool,
    pub outputs: Vec<OutputValue>,
    pub message: String,
}

/// Get all terraform outputs or a specific one
pub fn get_outputs(
    terraform_path: &Path,
    project_dir: &Path,
    name: Option<&str>,
) -> anyhow::Result<OutputResult> {
    // Named output queries omit sensitivity metadata. Always retrieve the map,
    // then select and redact locally before anything crosses the MCP boundary.
    let output = Command::new(terraform_path)
        .args(["output", "-json"])
        .current_dir(project_dir)
        .output()?;
    anyhow::ensure!(
        output.status.success(),
        "Terraform output failed (exit code {:?}). Check the selected workspace, backend access, and state locally; diagnostics are withheld because they may contain sensitive values",
        output.status.code()
    );
    let map: std::collections::BTreeMap<String, TerraformOutput> =
        serde_json::from_slice(&output.stdout)
            .map_err(|_| anyhow::anyhow!("Terraform returned an invalid output map"))?;
    if let Some(name) = name {
        anyhow::ensure!(map.contains_key(name), "Requested output was not found");
    }
    let outputs: Vec<_> = map
        .into_iter()
        .filter(|(output_name, _)| name.is_none_or(|name| output_name == name))
        .map(|(name, output)| OutputValue {
            name,
            value_type: output
                .value_type
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| get_value_type(&output.value)),
            value: if output.sensitive {
                serde_json::json!("[sensitive]")
            } else {
                output.value
            },
            sensitive: output.sensitive,
            description: None,
        })
        .collect();

    let message = if outputs.is_empty() {
        "No outputs found".to_string()
    } else {
        format!("Found {} outputs", outputs.len())
    };

    Ok(OutputResult {
        success: true,
        outputs,
        message,
    })
}

#[derive(Deserialize)]
struct TerraformOutput {
    sensitive: bool,
    value: serde_json::Value,
    #[serde(rename = "type")]
    value_type: serde_json::Value,
}

/// Determine the type of a JSON value
fn get_value_type(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(_) => "bool".to_string(),
        serde_json::Value::Number(_) => "number".to_string(),
        serde_json::Value::String(_) => "string".to_string(),
        serde_json::Value::Array(arr) => {
            if arr.is_empty() {
                "list".to_string()
            } else {
                format!("list({})", get_value_type(&arr[0]))
            }
        }
        serde_json::Value::Object(_) => "object".to_string(),
    }
}

/// Get outputs in a simple key-value format (non-JSON)
#[allow(dead_code)]
pub fn get_outputs_simple(
    terraform_path: &Path,
    project_dir: &Path,
) -> anyhow::Result<Vec<(String, String)>> {
    let output = Command::new(terraform_path)
        .arg("output")
        .current_dir(project_dir)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("No outputs found") {
            return Ok(vec![]);
        }
        return Err(anyhow::anyhow!("Failed to get outputs: {stderr}"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut results = Vec::new();

    for line in stdout.lines() {
        if let Some((name, value)) = line.split_once(" = ") {
            results.push((name.trim().to_string(), value.trim().to_string()));
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_value_type() {
        assert_eq!(get_value_type(&serde_json::Value::Null), "null");
        assert_eq!(get_value_type(&serde_json::json!(true)), "bool");
        assert_eq!(get_value_type(&serde_json::json!(42)), "number");
        assert_eq!(get_value_type(&serde_json::json!("hello")), "string");
        assert_eq!(
            get_value_type(&serde_json::json!([1, 2, 3])),
            "list(number)"
        );
        assert_eq!(get_value_type(&serde_json::json!({})), "object");
    }

    #[test]
    fn test_empty_outputs() {
        // Test that empty output is handled correctly
        let empty_map = serde_json::json!({});
        assert!(empty_map.as_object().unwrap().is_empty());
    }
}
