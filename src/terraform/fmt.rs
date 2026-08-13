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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatMode {
    Check,
    Diff,
    Write,
}

/// Run Terraform formatting in check, diff, or write mode.
pub fn format_files(
    terraform_path: &Path,
    project_dir: &Path,
    file: Option<&str>,
    mode: FormatMode,
) -> anyhow::Result<FormatResult> {
    let check_only = mode != FormatMode::Write;
    let show_diff = mode == FormatMode::Diff;
    let mut cmd = Command::new(terraform_path);
    cmd.arg("fmt");

    if check_only {
        cmd.arg("-check");
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

    if show_diff {
        for result in &mut file_results {
            let diff = format_diff(terraform_path, project_dir, &result.file)?;
            if !diff.trim().is_empty() {
                result.diff = Some(diff);
            }
        }
    }

    // Count unchanged files by listing all .tf files
    let all_tf_files = count_tf_files(project_dir);
    let files_unchanged = all_tf_files.saturating_sub(files_formatted);

    let success = output.status.success();

    let message = if check_only {
        if output.status.success() {
            "All files are properly formatted".to_string()
        } else {
            format!("{files_formatted} files need formatting")
        }
    } else if files_formatted > 0 {
        format!("Formatted {files_formatted} files")
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

fn format_diff(terraform_path: &Path, project_dir: &Path, file: &str) -> anyhow::Result<String> {
    let output = Command::new(terraform_path)
        .args(["fmt", "-write=false", "-diff", "-list=false", file])
        .current_dir(project_dir)
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() && stdout.is_empty() && !stderr.trim().is_empty() {
        anyhow::bail!("terraform fmt diff failed for {file}: {}", stderr.trim());
    }
    Ok(stdout)
}

/// Count .tf files in a directory (recursive)
fn count_tf_files(dir: &Path) -> usize {
    let mut count = 0;

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Skip hidden directories and common non-terraform directories
                if let Some(name) = path.file_name().and_then(|n| n.to_str())
                    && (name.starts_with('.') || name == "node_modules" || name == "vendor")
                {
                    continue;
                }
                count += count_tf_files(&path);
            } else if path.is_file()
                && let Some(ext) = path.extension()
                && ext == "tf"
            {
                count += 1;
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

    #[test]
    fn diff_mode_does_not_modify_files() {
        let Ok(terraform) = which::which("terraform") else {
            return;
        };
        let temp_dir = TempDir::new().unwrap();
        let file = temp_dir.path().join("main.tf");
        let original = "resource \"null_resource\" \"example\"{}\n";
        fs::write(&file, original).unwrap();

        let result = format_files(&terraform, temp_dir.path(), None, FormatMode::Diff).unwrap();

        assert!(!result.success);
        assert_eq!(fs::read_to_string(file).unwrap(), original);
        assert!(
            result.file_results.iter().any(|file| {
                file.diff
                    .as_deref()
                    .is_some_and(|diff| diff.contains("--- old/"))
            }),
            "expected terraform fmt diff output, got {result:#?}"
        );
    }
}
