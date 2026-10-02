use super::*;

#[tokio::test]
async fn discarding_a_plan_removes_its_files_and_frees_capacity() -> Result<()> {
    let executable = which::which("terraform")?;
    let project = tempfile::tempdir()?;
    std::fs::write(
        project.path().join("main.tf"),
        "output \"message\" { value = \"hello\" }",
    )?;
    let init = execution::run(&executable, project.path(), &["init", "-input=false"]).await?;
    execution::ensure_success(&init, "init")?;
    let mut plans = PlanStore::default();
    for _ in 0..64 {
        plans
            .create(&executable, project.path(), &PlanOptions::default())
            .await?;
    }
    let overflow = plans
        .create(&executable, project.path(), &PlanOptions::default())
        .await;
    assert!(overflow.is_err());
    let first = plans.list().remove(0);
    let path = plans
        .plans
        .get(&first.plan_id)
        .context("saved plan")?
        .directory
        .path()
        .to_path_buf();
    assert!(path.join("plan.tfplan").is_file());
    plans.discard(&first.plan_id)?;
    assert!(!path.exists());
    assert!(plans.read(&first.plan_id).is_err());
    plans
        .create(&executable, project.path(), &PlanOptions::default())
        .await?;
    assert_eq!(plans.list().len(), 64);
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn cancelled_apply_keeps_an_unknown_non_retryable_outcome() -> Result<()> {
    let executable = which::which("terraform")?;
    let project = tempfile::tempdir()?;
    std::fs::write(
        project.path().join("main.tf"),
        r#"resource "terraform_data" "slow" {
  provisioner "local-exec" { command = "echo started > started.txt; sleep 3" }
}"#,
    )?;
    let init = execution::run(&executable, project.path(), &["init", "-input=false"]).await?;
    execution::ensure_success(&init, "init")?;
    let mut plans = PlanStore::default();
    let plan = plans
        .create(&executable, project.path(), &PlanOptions::default())
        .await?;
    {
        let apply = plans.apply(&executable, project.path(), &plan.plan_id);
        tokio::pin!(apply);
        let started = async {
            while !project.path().join("started.txt").exists() {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        };
        tokio::select! {
            result = &mut apply => anyhow::bail!("Apply completed before cancellation: {result:?}"),
            result = tokio::time::timeout(std::time::Duration::from_secs(10), started) => { result?; }
        }
    }
    let snapshot = plans.read(&plan.plan_id)?;
    assert_eq!(snapshot.status, PlanStatus::OutcomeUnknown);
    assert!(snapshot.apply_result.is_none());
    let guidance = RecoveryGuidance::for_status(snapshot.status, false);
    assert!(guidance.state_may_have_changed);
    assert!(!guidance.next_steps.is_empty());
    assert!(
        plans
            .apply(&executable, project.path(), &plan.plan_id)
            .await
            .is_err()
    );
    Ok(())
}
