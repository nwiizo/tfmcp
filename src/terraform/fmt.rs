//! Terraform fmt operations for code formatting.

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

/// Format check result for a single file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileFormatResult {
    pub file: String,
    pub formatted: bool,
    pub diff: Option<String>,
}

/// Overall format result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatResult {
    pub success: bool,
    pub files_checked: i32,
    pub files_formatted: i32,
    pub files_unchanged: i32,
    pub file_results: Vec<FileFormatResult>,
    pub message: String,
}

/// Check formatting without making changes
pub fn check_format(
    terraform_path: &Path,
    project_dir: &Path,
    file: Option<&str>,
) -> anyhow::Result<FormatResult> {
    format_internal(terraform_path, project_dir, file, true, false)
}

/// Format files and show diff
pub fn format_with_diff(
    terraform_path: &Path,
    project_dir: &Path,
    file: Option<&str>,
) -> anyhow::Result<FormatResult> {
    format_internal(terraform_path, project_dir, file, false, true)
}

/// Format files in place
pub fn format_files(
    terraform_path: &Path,
    project_dir: &Path,
    file: Option<&str>,
) -> anyhow::Result<FormatResult> {
    format_internal(terraform_path, project_dir, file, false, false)
}

/// Internal format implementation
fn format_internal(
    terraform_path: &Path,
    project_dir: &Path,
    file: Option<&str>,
    check_only: bool,
    show_diff: bool,
) -> anyhow::Result<FormatResult> {
    let mut cmd = Command::new(terraform_path);
    cmd.arg("fmt");

    if check_only {
        cmd.arg("-check");
    }

    if show_diff {
        cmd.arg("-diff");
    }

    // List files that would be formatted
    cmd.arg("-list=true");

    // Recursive formatting
    cmd.arg("-recursive");

    // If a specific file is provided, use it
    if let Some(file_path) = file {
        cmd.arg(file_path);
    }

    cmd.current_dir(project_dir);

    let output = cmd.output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Parse the output
    let mut file_results = Vec::new();
    let mut files_formatted = 0;

    // stdout contains list of formatted/unformatted files
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // terraform fmt -list=true outputs filenames that were/would be formatted
        file_results.push(FileFormatResult {
            file: line.to_string(),
            formatted: true,
            diff: None,
        });
        files_formatted += 1;
    }

    // If we're doing a diff check, parse the diff output
    if show_diff && !stderr.is_empty() {
        // Diffs are written to stdout when using -diff
        if let Some(last_result) = file_results.last_mut() {
            last_result.diff = Some(stderr.to_string());
        }
    }

    // Count unchanged files by listing all .tf files
    let all_tf_files = count_tf_files(project_dir);
    let files_unchanged = all_tf_files.saturating_sub(files_formatted);

    let success = output.status.success() || (!check_only && output.status.code() == Some(0));

    let message = if check_only {
        if output.status.success() {
            "All files are properly formatted".to_string()
        } else {
            format!("{} files need formatting", files_formatted)
        }
    } else if files_formatted > 0 {
        format!("Formatted {} files", files_formatted)
    } else {
        "No files needed formatting".to_string()
    };

    Ok(FormatResult {
        success,
        files_checked: all_tf_files as i32,
        files_formatted: files_formatted as i32,
        files_unchanged: files_unchanged as i32,
        file_results,
        message,
    })
}

/// Count .tf files in a directory (recursive)
fn count_tf_files(dir: &Path) -> usize {
    let mut count = 0;

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Skip hidden directories and common non-terraform directories
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with('.') || name == "node_modules" || name == "vendor" {
                        continue;
                    }
                }
                count += count_tf_files(&path);
            } else if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "tf" {
                        count += 1;
                    }
                }
            }
        }
    }

    count
}

/// Get format style recommendations
#[allow(dead_code)]
pub fn get_format_recommendations() -> Vec<String> {
    vec![
        "Use 2-space indentation for nested blocks".to_string(),
        "Align equals signs in attribute assignments within a block".to_string(),
        "Use lowercase for resource types and attribute names".to_string(),
        "Place the opening brace on the same line as the block header".to_string(),
        "Use blank lines to separate logical groups of attributes".to_string(),
        "Order meta-arguments (count, for_each, lifecycle) before resource-specific arguments"
            .to_string(),
        "Keep line length under 120 characters for readability".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_count_tf_files() {
        let temp_dir = TempDir::new().unwrap();

        // Create some .tf files
        fs::write(temp_dir.path().join("main.tf"), "").unwrap();
        fs::write(temp_dir.path().join("variables.tf"), "").unwrap();
        fs::write(temp_dir.path().join("other.txt"), "").unwrap();

        let count = count_tf_files(temp_dir.path());
        assert_eq!(count, 2);
    }

    #[test]
    fn test_format_recommendations() {
        let recommendations = get_format_recommendations();
        assert!(!recommendations.is_empty());
        assert!(recommendations.iter().any(|r| r.contains("indentation")));
    }
}
