//! Terraform analysis tool orchestration and result rendering.

use super::{CallToolResult, McpError, TfMcpServer};
use crate::core::tfmcp::TfMcp;
use crate::terraform::model::core::TerraformAnalysis;
use crate::terraform::model::health::ModuleHealthAnalysis;
use crate::terraform::model::validation::GuidelineCheckResult;
use std::future::Future;

pub(super) enum AnalysisToolCall {
    Terraform,
    ModuleHealth,
}

impl AnalysisToolCall {
    fn metadata(&self) -> (&'static str, &'static str) {
        match self {
            Self::Terraform => ("analyze_terraform", "Analysis failed"),
            Self::ModuleHealth => ("analyze_module_health", "Module health analysis failed"),
        }
    }
}

fn guideline_summary(checks: GuidelineCheckResult) -> serde_json::Value {
    serde_json::json!({
        "compliance_score": checks.compliance_score,
        "providers_missing_version": checks.providers_missing_version,
        "variables_missing_type": checks.variables_missing_type.len(),
        "variables_missing_description": checks.variables_missing_description.len(),
        "outputs_missing_description": checks.outputs_missing_description.len()
    })
}

fn variable_quality(checks: GuidelineCheckResult) -> serde_json::Value {
    serde_json::json!({
        "variables_missing_type": checks.variables_missing_type,
        "variables_missing_description": checks.variables_missing_description,
        "any_type_usage": checks.any_type_usage,
        "outputs_missing_description": checks.outputs_missing_description
    })
}

fn terraform_analysis_json(
    analysis: TerraformAnalysis,
    guideline_summary: serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "project_directory": analysis.project_directory,
        "file_count": analysis.file_count,
        "resources": analysis.resources,
        "variables": analysis.variables,
        "outputs": analysis.outputs,
        "providers": analysis.providers,
        "guideline_summary": guideline_summary
    })
}

fn module_health_json(
    health: ModuleHealthAnalysis,
    variable_quality: serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "module_path": health.module_path,
        "health_score": health.health_score,
        "metrics": health.metrics,
        "issues": health.issues,
        "recommendations": health.recommendations,
        "cohesion_analysis": health.cohesion_analysis,
        "coupling_analysis": health.coupling_analysis,
        "variable_quality": variable_quality
    })
}

impl TfMcpServer {
    pub(super) async fn run_analysis_call(
        &self,
        call: AnalysisToolCall,
    ) -> Result<CallToolResult, McpError> {
        let (tool_name, error_prefix) = call.metadata();
        self.run_measured_tool_call(tool_name, error_prefix, self.analysis_call_value(call))
            .await
    }

    async fn analysis_call_value(
        &self,
        call: AnalysisToolCall,
    ) -> Result<serde_json::Value, String> {
        match call {
            AnalysisToolCall::Terraform => self.terraform_analysis_value().await,
            AnalysisToolCall::ModuleHealth => self.module_health_value().await,
        }
    }

    async fn analysis_value<T, Action, Render>(
        &self,
        action: Action,
        scan_render: fn(GuidelineCheckResult) -> serde_json::Value,
        render: Render,
    ) -> Result<serde_json::Value, String>
    where
        Action: for<'a> FnOnce(
            &'a TfMcp,
        ) -> std::pin::Pin<
            Box<dyn Future<Output = anyhow::Result<T>> + Send + 'a>,
        >,
        Render: FnOnce(T, serde_json::Value) -> serde_json::Value,
    {
        let tfmcp = self.tfmcp.read().await;
        let value = action(&tfmcp).await.map_err(|e| e.to_string())?;
        let scan_value = match tfmcp.run_security_scan().await {
            Ok(checks) => scan_render(checks),
            Err(_) => serde_json::Value::Null,
        };
        Ok(render(value, scan_value))
    }

    async fn terraform_analysis_value(&self) -> Result<serde_json::Value, String> {
        self.analysis_value(
            |tfmcp| Box::pin(tfmcp.get_terraform_analysis()),
            guideline_summary,
            terraform_analysis_json,
        )
        .await
    }

    async fn module_health_value(&self) -> Result<serde_json::Value, String> {
        self.analysis_value(
            |tfmcp| Box::pin(tfmcp.analyze_module_health()),
            variable_quality,
            module_health_json,
        )
        .await
    }
}
