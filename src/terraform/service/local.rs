//! Local change workflow: prepare, retain, authorize, and audit saved plans.
use super::TerraformService;
use crate::terraform::{
    plan_analyzer::{PlanAnalysis, RiskAssessment, RiskLevel},
    preflight::{self, ExecutionPreflight},
    saved_plan::{ApplyResult, PlanOptions, PlanSnapshot},
};

impl TerraformService {
    pub async fn get_plan(&self) -> anyhow::Result<String> {
        let snapshot = self.create_saved_plan(&PlanOptions::default()).await?;
        snapshot.analysis.to_plan_json()
    }

    pub async fn apply(&self, auto_approve: bool) -> anyhow::Result<String> {
        self.check_apply_permissions(auto_approve)?;
        anyhow::bail!(
            "A saved plan_id is required. Generate and review a plan with get_terraform_plan, then pass its plan_id to apply_terraform"
        );
    }

    fn check_apply_permissions(&self, auto_approve: bool) -> anyhow::Result<()> {
        if !self.security_manager.is_command_allowed("apply") {
            anyhow::bail!(
                "Apply operation blocked by security policy. Set TFMCP_ALLOW_DANGEROUS_OPS=true to enable."
            );
        }
        if auto_approve && !self.security_manager.is_auto_approve_allowed("apply") {
            anyhow::bail!(
                "Auto-approve for apply operation blocked by security policy. Set TFMCP_ALLOW_AUTO_APPROVE=true to enable."
            );
        }
        anyhow::ensure!(
            auto_approve,
            "Saved plans execute without a Terraform prompt. Explicitly set auto_approve=true after reviewing the plan"
        );
        self.security_manager
            .validate_directory(&self.project_directory)?;
        Ok(())
    }

    pub async fn create_saved_plan(&self, options: &PlanOptions) -> anyhow::Result<PlanSnapshot> {
        self.security_manager
            .validate_directory(&self.project_directory)?;
        for file in &options.var_files {
            let path = self.project_directory.join(file).canonicalize()?;
            anyhow::ensure!(
                !self.security_manager.is_file_blocked(&path),
                "Variable file access is blocked by security policy"
            );
        }
        self.saved_plans
            .lock()
            .await
            .create(&self.terraform_path, &self.project_directory, options)
            .await
    }

    pub async fn read_saved_plan(&self, plan_id: &str) -> anyhow::Result<PlanSnapshot> {
        self.saved_plans.lock().await.read(plan_id)
    }

    pub async fn apply_saved_plan(
        &self,
        plan_id: &str,
        auto_approve: bool,
    ) -> anyhow::Result<ApplyResult> {
        self.check_apply_permissions(auto_approve)?;
        let mut plans = self.saved_plans.lock().await;
        let snapshot = plans.read(plan_id)?;
        self.security_manager
            .check_resource_limit(snapshot.analysis.resource_changes.len())?;
        let result = plans
            .apply(&self.terraform_path, &self.project_directory, plan_id)
            .await;
        let success = result.as_ref().is_ok_and(|result| result.success);
        let error = match &result {
            Ok(result) if !result.success => Some(result.diagnostics.join("; ")),
            Err(error) => Some(error.to_string()),
            _ => None,
        };
        let audit_entry = self.security_manager.create_audit_entry(
            "apply",
            &self.project_directory.to_string_lossy(),
            &[
                "terraform".to_string(),
                "apply".to_string(),
                plan_id.to_string(),
            ],
            success,
            error,
            result
                .as_ref()
                .ok()
                .and_then(|result| result.managed_resources),
        );
        if let Err(error) = self.security_manager.log_audit_entry(audit_entry) {
            tracing::warn!(%error, "Failed to log apply audit entry");
        }
        result
    }

    pub async fn inspect_execution(&self) -> anyhow::Result<ExecutionPreflight> {
        self.security_manager
            .validate_directory(&self.project_directory)?;
        preflight::inspect(&self.terraform_path, &self.project_directory).await
    }

    /// Analyze a detailed saved plan, with optional risk scoring.
    pub async fn analyze_plan(&self, include_risk: bool) -> anyhow::Result<PlanAnalysis> {
        let mut analysis = self
            .create_saved_plan(&PlanOptions::default())
            .await?
            .analysis;
        if !include_risk {
            analysis.risk_assessment = RiskAssessment {
                level: RiskLevel::Low,
                score: 0,
                warnings: Vec::new(),
                recommendations: Vec::new(),
            };
        }
        Ok(analysis)
    }
}
