//! Real local Terraform operations through the stdio MCP server.
use anyhow::{Context, Result};
use rmcp::{
    ClientHandler, ServiceExt,
    model::{CallToolRequestParams, CallToolResult, ClientInfo},
};
use serde_json::{Value, json};
use std::process::Stdio;
use tokio::process::{Child, Command};

#[derive(Clone, Debug)]
struct Client;

impl ClientHandler for Client {
    fn get_info(&self) -> ClientInfo {
        ClientInfo::default()
    }
}

struct Session {
    client: rmcp::service::RunningService<rmcp::RoleClient, Client>,
    _child: Child,
    directory: tempfile::TempDir,
}

impl Session {
    async fn start(configuration: &str, allow_apply: bool) -> Result<Self> {
        let directory = tempfile::tempdir()?;
        tokio::fs::write(directory.path().join("main.tf"), configuration).await?;
        let mut child = Command::new(env!("CARGO_BIN_EXE_tfmcp"))
            .args([
                "--dir",
                &directory.path().to_string_lossy(),
                "mcp",
                "--toolsets",
                "all",
            ])
            .env("TFMCP_ALLOW_DANGEROUS_OPS", allow_apply.to_string())
            .env("TFMCP_ALLOW_AUTO_APPROVE", allow_apply.to_string())
            .env("TFMCP_AUDIT_LOG_FILE", directory.path().join("audit.log"))
            .env_remove("TF_CLI_ARGS")
            .env_remove("TF_CLI_ARGS_plan")
            .env_remove("TF_CLI_ARGS_apply")
            .env_remove("TF_WORKSPACE")
            .env_remove("TF_DATA_DIR")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()?;
        let stdout = child.stdout.take().context("server stdout")?;
        let stdin = child.stdin.take().context("server stdin")?;
        let client = Client.serve((stdout, stdin)).await?;
        Ok(Self {
            client,
            _child: child,
            directory,
        })
    }

    async fn call(&self, tool: &str, arguments: Value) -> Result<CallToolResult> {
        Ok(self
            .client
            .call_tool(
                CallToolRequestParams::new(tool.to_string())
                    .with_arguments(arguments.as_object().cloned().context("object arguments")?),
            )
            .await?)
    }

    async fn value(&self, tool: &str, arguments: Value) -> Result<Value> {
        let result = self.call(tool, arguments).await?;
        anyhow::ensure!(result.is_error != Some(true), "{tool}: {result:?}");
        result.structured_content.context("structured tool result")
    }
}

const CONFIGURATION: &str =
    "resource \"terraform_data\" \"example\" { input = \"reviewed-value\" }";

#[tokio::test]
async fn saved_plan_is_reviewed_and_applied_without_replanning() -> Result<()> {
    let session = Session::start(CONFIGURATION, true).await?;
    session.value("init_terraform", json!({})).await?;
    let plan = session.value("get_terraform_plan", json!({})).await?;
    let plan_id = plan["plan_id"].as_str().context("saved plan ID")?;
    let review = session
        .value("review_terraform_plan", json!({"plan_id": plan_id}))
        .await?;
    assert_eq!(review["plan_id"], plan_id);
    assert_eq!(review["summary"], "1 add, 0 change, 0 destroy, 0 replace");
    assert_eq!(review["decision"], "review_required");
    let summary = session
        .value("summarize_plan_for_pr", json!({"plan_id": plan_id}))
        .await?;
    assert_eq!(summary["plan_id"], plan_id);
    assert_eq!(summary["created_at"], review["created_at"]);
    // The saved plan, not this subsequent edit, must determine the apply.
    tokio::fs::write(
        session.directory.path().join("main.tf"),
        "resource \"terraform_data\" \"example\" { input = \"unreviewed-value\" }",
    )
    .await?;
    let apply = session
        .value(
            "apply_terraform",
            json!({"plan_id": plan_id, "auto_approve": true}),
        )
        .await?;
    assert_eq!(apply["state_verified"], true);
    let state: Value = serde_json::from_slice(
        &tokio::fs::read(session.directory.path().join("terraform.tfstate")).await?,
    )?;
    assert_eq!(
        state["resources"][0]["instances"][0]["attributes"]["input"]["value"],
        "reviewed-value"
    );
    let snapshot = session
        .value("get_terraform_plan", json!({"plan_id": plan_id}))
        .await?;
    assert_eq!(snapshot["status"], "applied");
    let repeat = session
        .call(
            "apply_terraform",
            json!({"plan_id": plan_id, "auto_approve": true}),
        )
        .await?;
    assert_eq!(
        repeat.is_error,
        Some(true),
        "a consumed plan must not execute twice"
    );
    session.client.cancel().await?;
    Ok(())
}

#[tokio::test]
async fn preflight_distinguishes_a_new_project_from_unreadable_state() -> Result<()> {
    let session = Session::start(CONFIGURATION, false).await?;
    session.value("init_terraform", json!({})).await?;
    let prepared = session.value("prepare_terraform_change", json!({})).await?;
    assert_eq!(
        prepared["ready"], true,
        "a built-in provider needs no provider lockfile: {prepared}"
    );
    assert_eq!(prepared["execution"]["state_status"], "absent");
    assert_eq!(prepared["execution"]["workspace"], "default");
    tokio::fs::write(
        session.directory.path().join("terraform.tfstate"),
        "invalid state",
    )
    .await?;
    let prepared = session.value("prepare_terraform_change", json!({})).await?;
    assert_eq!(prepared["ready"], false);
    assert_eq!(prepared["execution"]["state_status"], "unavailable");
    session.client.cancel().await?;
    Ok(())
}

#[tokio::test]
async fn plan_input_errors_are_actionable_and_sensitive_values_are_redacted() -> Result<()> {
    let session = Session::start(
        r#"
variable "message" {
  type = string
  sensitive = true
}
resource "terraform_data" "example" { input = var.message }
"#,
        false,
    )
    .await?;
    session.value("init_terraform", json!({})).await?;
    let missing = session.call("get_terraform_plan", json!({})).await?;
    assert_eq!(missing.is_error, Some(true));
    assert!(serde_json::to_string(&missing)?.contains("No value for required variable"));
    tokio::fs::write(
        session.directory.path().join("input.tfvars"),
        "message = \"must-not-leave-the-server\"",
    )
    .await?;
    let plan = session
        .value("get_terraform_plan", json!({"var_files": ["input.tfvars"]}))
        .await?;
    assert!(!serde_json::to_string(&plan)?.contains("must-not-leave-the-server"));
    assert!(
        plan["plan"]
            .as_str()
            .context("plan JSON")?
            .contains("[sensitive]")
    );
    let plan_id = plan["plan_id"].as_str().context("plan ID")?;
    let denied = session
        .call(
            "apply_terraform",
            json!({"plan_id": plan_id, "auto_approve": true}),
        )
        .await?;
    assert_eq!(denied.is_error, Some(true));
    assert!(!session.directory.path().join("terraform.tfstate").exists());
    session.client.cancel().await?;
    Ok(())
}

#[tokio::test]
async fn saved_plan_cannot_be_applied_to_another_workspace() -> Result<()> {
    let session = Session::start(CONFIGURATION, true).await?;
    session.value("init_terraform", json!({})).await?;
    let plan = session.value("get_terraform_plan", json!({})).await?;
    session
        .value(
            "terraform_workspace",
            json!({"action": "new", "name": "other"}),
        )
        .await?;
    let denied = session
        .call(
            "apply_terraform",
            json!({"plan_id": plan["plan_id"], "auto_approve": true}),
        )
        .await?;
    assert_eq!(denied.is_error, Some(true));
    assert!(
        !session
            .directory
            .path()
            .join("terraform.tfstate.d/other/terraform.tfstate")
            .exists()
    );
    session.client.cancel().await?;
    Ok(())
}

#[tokio::test]
async fn stale_plan_failure_is_reported_and_cannot_be_retried() -> Result<()> {
    let session = Session::start(CONFIGURATION, true).await?;
    session.value("init_terraform", json!({})).await?;
    let first = session.value("get_terraform_plan", json!({})).await?;
    let second = session.value("get_terraform_plan", json!({})).await?;
    for args in [
        json!({"auto_approve": true}),
        json!({"plan_id": first["plan_id"]}),
    ] {
        assert_eq!(
            session.call("apply_terraform", args).await?.is_error,
            Some(true)
        );
    }
    session
        .value(
            "apply_terraform",
            json!({"plan_id": first["plan_id"], "auto_approve": true}),
        )
        .await?;
    let failed = session
        .call(
            "apply_terraform",
            json!({"plan_id": second["plan_id"], "auto_approve": true}),
        )
        .await?;
    assert_eq!(failed.is_error, Some(true));
    let failed = failed.structured_content.context("structured failure")?;
    assert_eq!(failed["success"], false);
    assert_eq!(failed["exit_code"], 1);
    let snapshot = session
        .value("get_terraform_plan", json!({"plan_id": second["plan_id"]}))
        .await?;
    assert_eq!(snapshot["status"], "failed");
    assert_eq!(
        session
            .call(
                "apply_terraform",
                json!({"plan_id": second["plan_id"], "auto_approve": true})
            )
            .await?
            .is_error,
        Some(true)
    );
    session.client.cancel().await?;
    Ok(())
}

#[tokio::test]
async fn output_only_changes_are_reviewed_and_refresh_preview_does_not_write_state() -> Result<()> {
    let session = Session::start("output \"message\" { value = \"hello\" }", true).await?;
    session.value("init_terraform", json!({})).await?;
    let preparation = session.value("prepare_terraform_change", json!({})).await?;
    assert_eq!(
        preparation["ready"], true,
        "output-only project: {preparation}"
    );
    let plan = session.value("get_terraform_plan", json!({})).await?;
    assert_eq!(plan["has_changes"], true);
    let review = session
        .value("review_terraform_plan", json!({"plan_id": plan["plan_id"]}))
        .await?;
    assert_eq!(review["decision"], "review_required");
    assert_eq!(review["changed_outputs"], json!(["message"]));
    session
        .value(
            "apply_terraform",
            json!({"plan_id": plan["plan_id"], "auto_approve": true}),
        )
        .await?;
    let before = tokio::fs::read(session.directory.path().join("terraform.tfstate")).await?;
    let preview = session
        .value("get_terraform_plan", json!({"refresh_only": true}))
        .await?;
    assert_eq!(preview["refresh_only"], true);
    assert_eq!(preview["has_changes"], false);
    assert_eq!(
        before,
        tokio::fs::read(session.directory.path().join("terraform.tfstate")).await?
    );
    session.client.cancel().await?;
    Ok(())
}
