//! CI-friendly Terraform quality report types.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QualitySeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerraformQualityCheck {
    pub name: String,
    pub passed: bool,
    pub severity: QualitySeverity,
    pub summary: String,
    pub details: Vec<String>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerraformQualityReport {
    pub passed: bool,
    pub project_directory: String,
    pub summary: String,
    pub checks: Vec<TerraformQualityCheck>,
    pub markdown: String,
}

impl TerraformQualityCheck {
    pub fn new(
        name: impl Into<String>,
        passed: bool,
        severity: QualitySeverity,
        summary: impl Into<String>,
        details: Vec<String>,
        recommendations: Vec<String>,
    ) -> Self {
        Self {
            name: name.into(),
            passed,
            severity,
            summary: summary.into(),
            details,
            recommendations,
        }
    }

    pub fn passed(name: impl Into<String>, summary: impl Into<String>) -> Self {
        Self::new(
            name,
            true,
            QualitySeverity::Info,
            summary,
            Vec::new(),
            Vec::new(),
        )
    }
}

impl TerraformQualityReport {
    pub fn new(project_directory: String, checks: Vec<TerraformQualityCheck>) -> Self {
        let passed = checks.iter().all(|check| check.passed);
        let failed = checks.iter().filter(|check| !check.passed).count();
        let warnings = checks
            .iter()
            .filter(|check| check.severity == QualitySeverity::Warning)
            .count();
        let summary = if passed {
            format!("{} quality checks passed", checks.len())
        } else {
            format!(
                "{failed} quality checks failed, {warnings} warning checks passed with warnings"
            )
        };
        let markdown = render_markdown(passed, &project_directory, &summary, &checks);

        Self {
            passed,
            project_directory,
            summary,
            checks,
            markdown,
        }
    }
}

fn render_markdown(
    passed: bool,
    project_directory: &str,
    summary: &str,
    checks: &[TerraformQualityCheck],
) -> String {
    let mut lines = vec![
        "## Terraform Quality Report".to_string(),
        String::new(),
        format!("- Status: `{}`", if passed { "passed" } else { "failed" }),
        format!("- Project: `{project_directory}`"),
        format!("- Summary: {summary}"),
        String::new(),
        "| Check | Status | Summary |".to_string(),
        "| --- | --- | --- |".to_string(),
    ];

    for check in checks {
        lines.push(format!(
            "| {} | `{}` | {} |",
            check.name,
            if check.passed { "passed" } else { "failed" },
            check.summary.replace('|', "\\|")
        ));
    }

    for check in checks {
        if check.details.is_empty() && check.recommendations.is_empty() {
            continue;
        }
        lines.push(String::new());
        lines.push(format!("### {}", check.name));
        for detail in &check.details {
            lines.push(format!("- {detail}"));
        }
        for recommendation in &check.recommendations {
            lines.push(format!("- Recommendation: {recommendation}"));
        }
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_fails_when_any_check_fails() {
        let report = TerraformQualityReport::new(
            "/tmp/project".to_string(),
            vec![
                TerraformQualityCheck::passed("validate", "valid"),
                TerraformQualityCheck::new(
                    "lockfile",
                    false,
                    QualitySeverity::Error,
                    "missing lockfile",
                    vec![".terraform.lock.hcl is missing".to_string()],
                    vec!["Run terraform init".to_string()],
                ),
            ],
        );

        assert!(!report.passed);
        assert!(report.summary.contains("1 quality checks failed"));
        assert!(report.markdown.contains("Terraform Quality Report"));
        assert!(report.markdown.contains("Recommendation"));
    }
}
