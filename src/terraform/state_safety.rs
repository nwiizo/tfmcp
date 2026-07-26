//! Read-only state safety checks for Terraform change preparation.

use crate::terraform::providers::ProviderLockfileCheck;
use crate::terraform::state_analyzer::{DriftResult, HealthStatus, StateAnalysis};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSafetyInspection {
    pub project_directory: String,
    pub state_readable: bool,
    pub managed_resource_count: i32,
    pub drift_candidate_count: usize,
    pub critical_health_checks: usize,
    pub provider_lockfile_exists: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub recommendations: Vec<String>,
    pub markdown: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftCandidates {
    pub project_directory: String,
    pub candidates: Vec<DriftResult>,
    pub warnings: Vec<String>,
    pub markdown: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerraformChangePreparation {
    pub project_directory: String,
    pub ready: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub recommended_sequence: Vec<String>,
    pub markdown: String,
}

pub fn inspect_state_safety(
    project_dir: &Path,
    state: Result<StateAnalysis, String>,
    lockfile: ProviderLockfileCheck,
) -> StateSafetyInspection {
    let project_directory = project_dir.display().to_string();
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();
    let mut recommendations = Vec::new();

    let (state_readable, managed_resource_count, drift_candidate_count, critical_health_checks) =
        match state {
            Ok(analysis) => {
                let critical = analysis
                    .health_checks
                    .iter()
                    .filter(|check| check.status == HealthStatus::Critical)
                    .count();
                if !analysis.drift_results.is_empty() {
                    warnings.push(format!(
                        "{} drift candidate(s) detected",
                        analysis.drift_results.len()
                    ));
                    recommendations
                        .push("Review drift candidates before planning a change".to_string());
                }
                (
                    true,
                    analysis.total_resources,
                    analysis.drift_results.len(),
                    critical,
                )
            }
            Err(error) => {
                warnings.push(format!("State could not be read: {error}"));
                recommendations.push(
                    "Run terraform init and ensure the selected workspace has readable state"
                        .to_string(),
                );
                (false, 0, 0, 0)
            }
        };

    if !lockfile.lockfile_exists {
        blockers.push("Provider lockfile .terraform.lock.hcl is missing".to_string());
        recommendations.extend(lockfile.recommendations.clone());
    }
    warnings.extend(lockfile.warnings.clone());

    let markdown = safety_markdown(
        "Terraform State Safety",
        &project_directory,
        &blockers,
        &warnings,
        &recommendations,
    );

    StateSafetyInspection {
        project_directory,
        state_readable,
        managed_resource_count,
        drift_candidate_count,
        critical_health_checks,
        provider_lockfile_exists: lockfile.lockfile_exists,
        blockers,
        warnings,
        recommendations,
        markdown,
    }
}

pub fn drift_candidates(
    project_dir: &Path,
    state: Result<StateAnalysis, String>,
) -> DriftCandidates {
    let project_directory = project_dir.display().to_string();
    let mut warnings = Vec::new();
    let candidates = match state {
        Ok(analysis) => analysis.drift_results,
        Err(error) => {
            warnings.push(format!("State could not be read: {error}"));
            Vec::new()
        }
    };

    let markdown = drift_markdown(&project_directory, &candidates, &warnings);
    DriftCandidates {
        project_directory,
        candidates,
        warnings,
        markdown,
    }
}

pub fn prepare_change(inspection: StateSafetyInspection) -> TerraformChangePreparation {
    let mut recommended_sequence = vec![
        "terraform fmt -check".to_string(),
        "terraform validate".to_string(),
        "check_provider_lockfile".to_string(),
        "review_terraform_plan".to_string(),
    ];
    if inspection.drift_candidate_count > 0 {
        recommended_sequence.insert(0, "detect_drift_candidates".to_string());
    }

    let ready = inspection.blockers.is_empty();
    let markdown = safety_markdown(
        "Terraform Change Preparation",
        &inspection.project_directory,
        &inspection.blockers,
        &inspection.warnings,
        &recommended_sequence,
    );

    TerraformChangePreparation {
        project_directory: inspection.project_directory,
        ready,
        blockers: inspection.blockers,
        warnings: inspection.warnings,
        recommended_sequence,
        markdown,
    }
}

fn safety_markdown(
    title: &str,
    project_directory: &str,
    blockers: &[String],
    warnings: &[String],
    recommendations: &[String],
) -> String {
    let mut lines = vec![
        format!("## {title}"),
        String::new(),
        format!("- Project: `{project_directory}`"),
        format!("- Blockers: {}", blockers.len()),
        format!("- Warnings: {}", warnings.len()),
    ];
    push_markdown_section(&mut lines, "Blockers", blockers);
    push_markdown_section(&mut lines, "Warnings", warnings);
    push_markdown_section(&mut lines, "Recommendations", recommendations);
    lines.join("\n")
}

fn drift_markdown(
    project_directory: &str,
    candidates: &[DriftResult],
    warnings: &[String],
) -> String {
    let mut lines = vec![
        "## Terraform Drift Candidates".to_string(),
        String::new(),
        format!("- Project: `{project_directory}`"),
        format!("- Candidates: {}", candidates.len()),
    ];
    push_markdown_section(&mut lines, "Warnings", warnings);
    if !candidates.is_empty() {
        lines.push(String::new());
        lines.push("### Candidates".to_string());
        for candidate in candidates {
            lines.push(format!(
                "- `{}`: {:?} ({})",
                candidate.address, candidate.drift_type, candidate.resource_type
            ));
        }
    }
    lines.join("\n")
}

fn push_markdown_section(lines: &mut Vec<String>, title: &str, items: &[String]) {
    if items.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push(format!("### {title}"));
    for item in items {
        lines.push(format!("- {item}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepare_change_blocks_on_missing_lockfile() {
        let inspection = inspect_state_safety(
            Path::new("/tmp/project"),
            Err("state unavailable".to_string()),
            ProviderLockfileCheck {
                lockfile_exists: false,
                provider_count: 0,
                locked_providers: Vec::new(),
                warnings: vec!["missing lockfile".to_string()],
                recommendations: vec!["Run terraform init".to_string()],
            },
        );
        let preparation = prepare_change(inspection);

        assert!(!preparation.ready);
        assert!(
            preparation
                .markdown
                .contains("Terraform Change Preparation")
        );
        assert!(preparation.blockers[0].contains("Provider lockfile"));
    }
}
