//! Render several views of the same saved local plan.
use crate::core::tfmcp::TfMcp;
use crate::mcp::types::PlanInput;
use crate::terraform::{
    plan_analyzer::{RiskAssessment, RiskLevel},
    plan_review,
    saved_plan::PlanOptions,
};

pub(super) enum PlanView {
    Output,
    Analysis(bool),
    Review,
    PrSummary,
}

pub(super) async fn plan_value(
    tfmcp: &TfMcp,
    input: PlanInput,
    view: PlanView,
) -> anyhow::Result<serde_json::Value> {
    let snapshot = tfmcp
        .saved_plan(
            input.plan_id.as_deref(),
            &PlanOptions {
                var_files: input.var_files,
                replace: input.replace,
                refresh_only: input.refresh_only,
            },
        )
        .await?;
    let mut analysis = snapshot.analysis;
    let mut value = match view {
        PlanView::Output => {
            serde_json::json!({"plan": analysis.to_plan_json()?, "has_changes": snapshot.has_changes})
        }
        PlanView::Analysis(include_risk) => {
            if !include_risk {
                analysis.risk_assessment = RiskAssessment {
                    level: RiskLevel::Low,
                    score: 0,
                    warnings: Vec::new(),
                    recommendations: Vec::new(),
                };
            }
            serde_json::to_value(analysis)?
        }
        PlanView::Review => serde_json::to_value(plan_review::review_plan(&analysis))?,
        PlanView::PrSummary => serde_json::to_value(plan_review::summarize_plan_for_pr(&analysis))?,
    };
    value["plan_id"] = serde_json::json!(snapshot.plan_id);
    value["target"] = serde_json::to_value(snapshot.target)?;
    value["created_at"] = serde_json::to_value(snapshot.created_at)?;
    value["status"] = serde_json::to_value(snapshot.status)?;
    value["refresh_only"] = serde_json::json!(snapshot.refresh_only);
    Ok(value)
}
