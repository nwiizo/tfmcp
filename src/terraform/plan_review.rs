use crate::terraform::plan_analyzer::{PlanAnalysis, ResourceChange, RiskLevel};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlanReviewDecision {
    Approve,
    ReviewRequired,
    Block,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanReview {
    pub decision: PlanReviewDecision,
    pub summary: String,
    pub risk_level: RiskLevel,
    pub risk_score: i32,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub recommendations: Vec<String>,
    pub destructive_changes: Vec<String>,
    pub replacement_changes: Vec<String>,
    pub markdown: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanPrSummary {
    pub title: String,
    pub markdown: String,
    pub risk_level: RiskLevel,
    pub change_counts: serde_json::Value,
}

pub fn review_plan(analysis: &PlanAnalysis) -> PlanReview {
    let destructive_changes = changes_with_actions(&analysis.resource_changes, &["delete"]);
    let replacement_changes = changes_with_actions(
        &analysis.resource_changes,
        &["replace", "create_delete", "delete_create"],
    );

    let mut blockers = Vec::new();
    if analysis.risk_assessment.level == RiskLevel::Critical {
        blockers.push("Critical risk score requires explicit human approval".to_string());
    }
    if analysis.summary.destroy > 0 {
        blockers.push(format!(
            "{} resource(s) will be destroyed",
            analysis.summary.destroy
        ));
    }

    let mut warnings = analysis.risk_assessment.warnings.clone();
    if analysis.summary.replace > 0 {
        warnings.push(format!(
            "{} resource(s) will be replaced",
            analysis.summary.replace
        ));
    }

    let decision = if !blockers.is_empty() {
        PlanReviewDecision::Block
    } else if analysis.risk_assessment.level == RiskLevel::High || !warnings.is_empty() {
        PlanReviewDecision::ReviewRequired
    } else {
        PlanReviewDecision::Approve
    };

    let summary = format!(
        "{} add, {} change, {} destroy, {} replace",
        analysis.summary.add,
        analysis.summary.change,
        analysis.summary.destroy,
        analysis.summary.replace
    );

    let markdown = render_plan_review_markdown(
        &decision,
        &summary,
        analysis,
        &blockers,
        &warnings,
        &destructive_changes,
        &replacement_changes,
    );

    PlanReview {
        decision,
        summary,
        risk_level: analysis.risk_assessment.level.clone(),
        risk_score: analysis.risk_assessment.score,
        blockers,
        warnings,
        recommendations: analysis.risk_assessment.recommendations.clone(),
        destructive_changes,
        replacement_changes,
        markdown,
    }
}

pub fn summarize_plan_for_pr(analysis: &PlanAnalysis) -> PlanPrSummary {
    let review = review_plan(analysis);
    let title = format!(
        "Terraform plan: {} add, {} change, {} destroy, {} replace",
        analysis.summary.add,
        analysis.summary.change,
        analysis.summary.destroy,
        analysis.summary.replace
    );

    PlanPrSummary {
        title,
        markdown: review.markdown,
        risk_level: review.risk_level,
        change_counts: serde_json::json!({
            "add": analysis.summary.add,
            "change": analysis.summary.change,
            "destroy": analysis.summary.destroy,
            "replace": analysis.summary.replace,
            "no_op": analysis.summary.no_op
        }),
    }
}

fn changes_with_actions(changes: &[ResourceChange], actions: &[&str]) -> Vec<String> {
    changes
        .iter()
        .filter(|change| actions.iter().any(|action| change.action.contains(action)))
        .map(|change| change.address.clone())
        .collect()
}

fn render_plan_review_markdown(
    decision: &PlanReviewDecision,
    summary: &str,
    analysis: &PlanAnalysis,
    blockers: &[String],
    warnings: &[String],
    destructive_changes: &[String],
    replacement_changes: &[String],
) -> String {
    let mut lines = vec![
        "## Terraform Plan Review".to_string(),
        String::new(),
        format!("- Decision: `{:?}`", decision),
        format!(
            "- Risk: `{:?}` ({})",
            analysis.risk_assessment.level, analysis.risk_assessment.score
        ),
        format!("- Changes: {}", summary),
    ];

    push_section(&mut lines, "Blockers", blockers);
    push_section(&mut lines, "Warnings", warnings);
    push_section(
        &mut lines,
        "Recommendations",
        &analysis.risk_assessment.recommendations,
    );
    push_section(&mut lines, "Destructive Changes", destructive_changes);
    push_section(&mut lines, "Replacement Changes", replacement_changes);

    lines.join("\n")
}

fn push_section(lines: &mut Vec<String>, title: &str, items: &[String]) {
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
    use crate::terraform::plan_analyzer::{
        ChangeSummary, DependencyImpact, PlanAnalysis, RiskAssessment,
    };

    #[test]
    fn review_blocks_destroy() {
        let analysis = plan_analysis("delete", RiskLevel::Medium);
        let review = review_plan(&analysis);

        assert_eq!(review.decision, PlanReviewDecision::Block);
        assert_eq!(review.destructive_changes, vec!["aws_s3_bucket.data"]);
        assert!(review.markdown.contains("Destructive Changes"));
    }

    #[test]
    fn pr_summary_contains_change_counts() {
        let analysis = plan_analysis("create", RiskLevel::Low);
        let summary = summarize_plan_for_pr(&analysis);

        assert!(summary.title.contains("1 add"));
        assert_eq!(summary.change_counts["add"], 1);
        assert!(summary.markdown.contains("Terraform Plan Review"));
    }

    fn plan_analysis(action: &str, risk_level: RiskLevel) -> PlanAnalysis {
        PlanAnalysis {
            summary: ChangeSummary {
                add: i32::from(action == "create"),
                change: i32::from(action == "update"),
                destroy: i32::from(action == "delete"),
                replace: i32::from(action == "replace"),
                no_op: 0,
            },
            resource_changes: vec![ResourceChange {
                address: "aws_s3_bucket.data".to_string(),
                resource_type: "aws_s3_bucket".to_string(),
                provider: "registry.terraform.io/hashicorp/aws".to_string(),
                action: action.to_string(),
                before: None,
                after: None,
                after_unknown: None,
            }],
            risk_assessment: RiskAssessment {
                level: risk_level,
                score: 20,
                warnings: Vec::new(),
                recommendations: vec!["Review state-sensitive resources".to_string()],
            },
            dependency_impacts: Vec::<DependencyImpact>::new(),
            terraform_version: Some("1.6.0".to_string()),
            format_version: Some("1.2".to_string()),
        }
    }
}
