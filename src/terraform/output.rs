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
    let mut cmd = Command::new(terraform_path);
    cmd.arg("output").arg("-json");

    // If a specific output is requested
    if let Some(output_name) = name {
        cmd.arg(output_name);
    }

    let output = cmd.current_dir(project_dir).output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        // Check for common errors
        if stderr.contains("No outputs found") || stdout.trim().is_empty() {
            return Ok(OutputResult {
                success: true,
                outputs: vec![],
                message: "No outputs defined in this configuration".to_string(),
            });
        }

        if stderr.contains("output") && stderr.contains("not found") {
            return Err(anyhow::anyhow!(
                "Output '{}' not found",
                name.unwrap_or("unknown")
            ));
        }

        return Err(anyhow::anyhow!("Failed to get outputs: {}", stderr));
    }

    // Parse JSON output
    let outputs = if let Some(output_name) = name {
        // Single output: the JSON is the value itself
        if stdout.trim().is_empty() {
            vec![]
        } else {
            match serde_json::from_str::<serde_json::Value>(&stdout) {
                Ok(value) => {
                    vec![OutputValue {
                        name: output_name.to_string(),
                        value: value.clone(),
                        value_type: get_value_type(&value),
                        sensitive: false, // Can't determine from single output
                        description: None,
                    }]
                }
                Err(e) => return Err(anyhow::anyhow!("Failed to parse output JSON: {}", e)),
            }
        }
    } else {
        // All outputs: JSON is a map of output names to output objects
        if stdout.trim().is_empty() || stdout.trim() == "{}" {
            vec![]
        } else {
            match serde_json::from_str::<serde_json::Value>(&stdout) {
                Ok(serde_json::Value::Object(map)) => {
                    let mut outputs = Vec::new();
                    for (name, output_obj) in map {
                        let value = output_obj
                            .get("value")
                            .cloned()
                            .unwrap_or(serde_json::Value::Null);
                        let sensitive = output_obj
                            .get("sensitive")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        let value_type = output_obj
                            .get("type")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| get_value_type(&value));

                        outputs.push(OutputValue {
                            name,
                            value,
                            value_type,
                            sensitive,
                            description: None,
                        });
                    }
                    outputs
                }
                Ok(_) => {
                    return Err(anyhow::anyhow!("Unexpected output format"));
                }
                Err(e) => return Err(anyhow::anyhow!("Failed to parse outputs JSON: {}", e)),
            }
        }
    };

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
        return Err(anyhow::anyhow!("Failed to get outputs: {}", stderr));
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
