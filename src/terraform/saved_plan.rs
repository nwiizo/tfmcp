//! Saved plans are opaque server-owned artifacts, bound to their CLI target.
use super::{execution, plan_analyzer, preflight};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Default)]
pub struct PlanOptions {
    pub var_files: Vec<String>,
    pub replace: Vec<String>,
    pub refresh_only: bool,
    pub destroy: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanStatus {
    Ready,
    OutcomeUnknown,
    Applied,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlanSnapshot {
    pub plan_id: String,
    pub target: preflight::ExecutionTarget,
    pub created_at: DateTime<Utc>,
    pub status: PlanStatus,
    pub has_changes: bool,
    pub refresh_only: bool,
    pub destroy: bool,
    pub analysis: plan_analyzer::PlanAnalysis,
    pub apply_result: Option<ApplyResult>,
}

#[derive(Debug, Serialize)]
pub struct PlanSummary {
    pub plan_id: String,
    pub target: preflight::ExecutionTarget,
    pub created_at: DateTime<Utc>,
    pub status: PlanStatus,
    pub destroy: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecoveryGuidance {
    pub state_may_have_changed: bool,
    pub next_steps: Vec<&'static str>,
}

impl RecoveryGuidance {
    pub fn for_status(status: PlanStatus, state_verified: bool) -> Self {
        let next_steps = match status {
            PlanStatus::Ready => Vec::new(),
            PlanStatus::Applied if state_verified => Vec::new(),
            PlanStatus::Applied => vec![
                "Apply completed, but state verification is incomplete. Inspect state and outputs for the recorded target before making another change.",
            ],
            PlanStatus::Failed | PlanStatus::OutcomeUnknown => vec![
                "Do not retry this plan: some changes may already have been applied.",
                "Confirm Terraform and any provider or provisioner operations have stopped; do not force-unlock an active operation.",
                "Select the recorded project, workspace, and backend, then inspect state and the actual resources.",
                "Resolve the reported problem, generate a new plan, and review it before applying.",
            ],
        };
        Self {
            state_may_have_changed: status != PlanStatus::Ready,
            next_steps,
        }
    }
}

struct SavedPlan {
    directory: tempfile::TempDir,
    hash: Vec<u8>,
    target: preflight::TargetSnapshot,
    snapshot: PlanSnapshot,
}

#[derive(Default)]
pub(super) struct PlanStore {
    plans: HashMap<String, SavedPlan>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApplyResult {
    pub plan_id: String,
    pub target: preflight::ExecutionTarget,
    pub success: bool,
    pub status: PlanStatus,
    pub exit_code: Option<i32>,
    pub diagnostics: Vec<String>,
    pub state_verified: bool,
    pub managed_resources: Option<usize>,
    pub output: String,
    pub recovery: RecoveryGuidance,
}

impl PlanStore {
    pub async fn create(
        &mut self,
        executable: &Path,
        directory: &Path,
        options: &PlanOptions,
    ) -> Result<PlanSnapshot> {
        anyhow::ensure!(
            !options.refresh_only || options.replace.is_empty(),
            "refresh_only cannot be combined with replacement requests"
        );
        anyhow::ensure!(
            !options.destroy || (!options.refresh_only && options.replace.is_empty()),
            "destroy cannot be combined with refresh_only or replacement requests"
        );
        // nwiizo-coding-style: plans live for one server process, with a bounded cache;
        // add durable retention when restart-resumable local execution is required.
        anyhow::ensure!(
            self.plans.len() < 64,
            "Saved plan limit reached (64); use list_terraform_plans and discard_terraform_plan to remove unneeded plans"
        );
        let target = preflight::target_snapshot(executable, directory).await?;
        let temporary = tempfile::Builder::new().prefix("tfmcp-plan-").tempdir()?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(temporary.path(), std::fs::Permissions::from_mode(0o700))?;
        }
        let plan_path = temporary.path().join("plan.tfplan");
        let mut args = vec![
            "plan".to_string(),
            "-input=false".to_string(),
            "-json".to_string(),
            "-detailed-exitcode".to_string(),
            "-lock-timeout=30s".to_string(),
            format!("-out={}", plan_path.display()),
        ];
        if options.refresh_only {
            args.push("-refresh-only".to_string());
        }
        if options.destroy {
            args.push("-destroy".to_string());
        }
        for file in &options.var_files {
            let path = directory
                .join(file)
                .canonicalize()
                .context("Variable file is unavailable")?;
            anyhow::ensure!(path.is_file(), "Variable file must be a regular file");
            args.push(format!("-var-file={}", path.display()));
        }
        for address in &options.replace {
            anyhow::ensure!(
                !address.trim().is_empty() && !address.contains('\0'),
                "Replacement address must not be empty"
            );
            args.push(format!("-replace={address}"));
        }
        let arguments: Vec<&str> = args.iter().map(String::as_str).collect();
        let output = execution::run(executable, directory, &arguments).await?;
        if !matches!(output.status.code(), Some(0 | 2)) {
            execution::ensure_success(&output, "plan")?;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&plan_path, std::fs::Permissions::from_mode(0o600))?;
        }
        let shown = execution::run(
            executable,
            directory,
            &["show", "-json", &plan_path.to_string_lossy()],
        )
        .await?;
        execution::ensure_success(&shown, "show")?;
        let analysis = plan_analyzer::analyze_plan(std::str::from_utf8(&shown.stdout)?, true)?;
        let current = preflight::target_snapshot(executable, directory).await?;
        anyhow::ensure!(
            target == current,
            "Terraform target changed while planning; select the intended target and create a new plan"
        );
        let plan_id = temporary
            .path()
            .file_name()
            .context("Plan ID unavailable")?
            .to_string_lossy()
            .to_string();
        let snapshot = PlanSnapshot {
            plan_id: plan_id.clone(),
            target: target.target.clone(),
            created_at: Utc::now(),
            status: PlanStatus::Ready,
            has_changes: output.status.code() == Some(2),
            refresh_only: options.refresh_only,
            destroy: options.destroy,
            analysis,
            apply_result: None,
        };
        let hash = preflight::fingerprint(&std::fs::read(&plan_path)?);
        self.plans.insert(
            plan_id,
            SavedPlan {
                directory: temporary,
                hash,
                target,
                snapshot: snapshot.clone(),
            },
        );
        Ok(snapshot)
    }

    pub fn read(&self, plan_id: &str) -> Result<PlanSnapshot> {
        Ok(self
            .plans
            .get(plan_id)
            .context("Unknown or expired plan_id; generate a new plan")?
            .snapshot
            .clone())
    }

    pub fn list(&self) -> Vec<PlanSummary> {
        let mut plans: Vec<_> = self
            .plans
            .values()
            .map(|plan| {
                let snapshot = &plan.snapshot;
                PlanSummary {
                    plan_id: snapshot.plan_id.clone(),
                    target: snapshot.target.clone(),
                    created_at: snapshot.created_at,
                    status: snapshot.status,
                    destroy: snapshot.destroy,
                }
            })
            .collect();
        plans.sort_by(|a, b| {
            a.created_at
                .cmp(&b.created_at)
                .then(a.plan_id.cmp(&b.plan_id))
        });
        plans
    }

    pub fn discard(&mut self, plan_id: &str) -> Result<()> {
        let plan = self
            .plans
            .get(plan_id)
            .context("Unknown or expired plan_id; use list_terraform_plans")?;
        std::fs::remove_dir_all(plan.directory.path())
            .context("Could not remove saved plan files")?;
        self.plans.remove(plan_id);
        Ok(())
    }

    pub async fn apply(
        &mut self,
        executable: &Path,
        directory: &Path,
        plan_id: &str,
    ) -> Result<ApplyResult> {
        let plan = self
            .plans
            .get_mut(plan_id)
            .context("Unknown or expired plan_id; generate a new plan")?;
        anyhow::ensure!(
            plan.snapshot.status == PlanStatus::Ready,
            "Plan has already been attempted; inspect state and generate a new plan before retrying"
        );
        let target = preflight::target_snapshot(executable, directory).await?;
        anyhow::ensure!(
            target == plan.target,
            "Plan target, workspace, backend, Terraform version, or provider lockfile changed; generate a new plan for the selected target"
        );
        let path = plan.directory.path().join("plan.tfplan");
        anyhow::ensure!(
            preflight::fingerprint(&std::fs::read(&path)?) == plan.hash,
            "Saved plan contents changed; generate a new plan"
        );
        // If the request is cancelled, the write may have partially completed.
        // Keep it non-retryable even when no final process result is observed.
        plan.snapshot.status = PlanStatus::OutcomeUnknown;
        let execution = execution::run(
            executable,
            directory,
            &[
                "apply",
                "-input=false",
                "-json",
                "-lock-timeout=30s",
                &path.to_string_lossy(),
            ],
        )
        .await;
        let (success, exit_code, mut diagnostics) = match execution {
            Ok(output) => (
                output.status.success(),
                output.status.code(),
                execution::diagnostics(&output),
            ),
            Err(error) => (false, None, vec![error.to_string()]),
        };
        plan.snapshot.status = match (success, exit_code) {
            (true, _) => PlanStatus::Applied,
            (false, Some(_)) => PlanStatus::Failed,
            (false, None) => PlanStatus::OutcomeUnknown,
        };
        let mut resources = None;
        if success {
            match execution::run(executable, directory, &["state", "list"]).await {
                Ok(state) if state.status.success() => {
                    let stdout = String::from_utf8_lossy(&state.stdout);
                    let addresses: std::collections::HashSet<_> = stdout.lines().filter(|line| !line.trim().is_empty()).collect();
                    let matches_plan = plan.snapshot.analysis.resource_changes.iter().all(|change| {
                        if change.action == "delete" || change.action == "forget" {
                            !addresses.contains(change.address.as_str())
                        } else {
                            addresses.contains(change.address.as_str())
                        }
                    });
                    if matches_plan {
                        resources = Some(addresses.len());
                    } else {
                        diagnostics.push("Apply succeeded, but state resource addresses do not match the saved plan; inspect state before further operations".to_string());
                    }
                }
                _ => diagnostics.push("Apply succeeded, but state verification failed; inspect state before further operations".to_string()),
            }
        }
        let result = ApplyResult {
            plan_id: plan_id.to_string(),
            target: plan.snapshot.target.clone(),
            success,
            status: plan.snapshot.status,
            exit_code,
            diagnostics,
            state_verified: resources.is_some(),
            managed_resources: resources,
            output: if success {
                "Saved Terraform plan applied"
            } else if plan.snapshot.status == PlanStatus::OutcomeUnknown {
                "Apply outcome could not be confirmed; inspect state before creating a new plan"
            } else {
                "Saved plan apply failed; inspect state before creating a new plan"
            }
            .to_string(),
            recovery: RecoveryGuidance::for_status(plan.snapshot.status, resources.is_some()),
        };
        plan.snapshot.apply_result = Some(result.clone());
        Ok(result)
    }
}

#[cfg(test)]
mod tests;
