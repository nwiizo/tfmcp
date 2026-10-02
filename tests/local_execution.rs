//! Real local Terraform operations through the stdio MCP server.
use anyhow::{Context, Result};
use rmcp::{
    ClientHandler, ServiceExt,
    model::{CallToolRequestParams, CallToolResult, ClientConfig},
};
use serde_json::{Value, json};
use std::process::Stdio;
use tokio::process::{Child, Command};

#[derive(Clone, Debug)]
struct Client;

impl ClientHandler for Client {
    fn get_info(&self) -> ClientConfig {
        ClientConfig::default()
    }
}

struct Session {
    client: rmcp::service::RunningService<rmcp::RoleClient, Client>,
    _child: Child,
    directory: tempfile::TempDir,
}

impl Session {
    async fn start(configuration: &str, allow_apply: bool) -> Result<Self> {
        Self::start_with_options(configuration, allow_apply, false, 900, false).await
    }

    async fn start_with_options(
        configuration: &str,
        allow_apply: bool,
        allow_delete: bool,
        timeout_seconds: u64,
        trace_terraform: bool,
    ) -> Result<Self> {
        let directory = tempfile::tempdir()?;
        let home = directory.path().join("home");
        tokio::fs::create_dir(&home).await?;
        tokio::fs::write(directory.path().join("main.tf"), configuration).await?;
        let mut child = Command::new(env!("CARGO_BIN_EXE_tfmcp"))
            .args([
                "--dir",
                &directory.path().to_string_lossy(),
                "mcp",
                "--toolsets",
                "all",
            ])
            .env_clear()
            .env("PATH", std::env::var_os("PATH").context("test PATH")?)
            .envs(std::env::var_os("SystemRoot").map(|value| ("SystemRoot", value)))
            .env("TFMCP_ALLOW_DANGEROUS_OPS", allow_apply.to_string())
            .env("TFMCP_ALLOW_AUTO_APPROVE", allow_apply.to_string())
            .env("TFMCP_DELETE_ENABLED", allow_delete.to_string())
            .env("TFMCP_COMMAND_TIMEOUT_SECONDS", timeout_seconds.to_string())
            .env("HOME", &home)
            .env("USERPROFILE", &home)
            .env("TF_CLI_CONFIG_FILE", home.join("terraform.rc"))
            .env("TFMCP_AUDIT_LOG_FILE", directory.path().join("audit.log"))
            .envs(trace_terraform.then_some(("TF_LOG", "DEBUG")))
            .envs(trace_terraform.then(|| ("TF_LOG_PATH", directory.path().join("terraform.log"))))
            .env_remove("TFE_TOKEN")
            .env_remove("TFE_ADDRESS")
            .env_remove("TF_CLI_ARGS")
            .env_remove("TF_CLI_ARGS_plan")
            .env_remove("TF_CLI_ARGS_apply")
            .env_remove("TF_WORKSPACE")
            .env_remove("TF_DATA_DIR")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::from(std::fs::File::create(
                directory.path().join("server.log"),
            )?))
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

    async fn audit_entries(&self) -> Result<Vec<tfmcp::shared::security::AuditLogEntry>> {
        tokio::fs::read_to_string(self.directory.path().join("audit.log"))
            .await?
            .lines()
            .map(|line| serde_json::from_str(line).map_err(Into::into))
            .collect()
    }
}

const CONFIGURATION: &str =
    "resource \"terraform_data\" \"example\" { input = \"reviewed-value\" }";

#[tokio::test]
async fn audit_write_failure_is_logged_on_stderr_without_breaking_stdio() -> Result<()> {
    let session = Session::start(CONFIGURATION, true).await?;
    // A real filesystem failure must produce a stderr warning, not JSON-RPC data.
    tokio::fs::create_dir(session.directory.path().join("audit.log")).await?;
    session.value("init_terraform", json!({})).await?;
    let plan = session.value("get_terraform_plan", json!({})).await?;
    let result = session
        .value(
            "apply_terraform",
            json!({"plan_id": plan["plan_id"], "auto_approve": true}),
        )
        .await?;
    assert_eq!(result["success"], true);
    session.value("list_terraform_plans", json!({})).await?;
    let log = tokio::fs::read_to_string(session.directory.path().join("server.log")).await?;
    assert!(log.contains("Failed to log apply audit entry"));
    assert!(
        !log.contains("DEBUG") && !log.contains("TerraformService initialized"),
        "info logging must suppress debug messages"
    );
    assert!(!log.contains("reviewed-value"));
    session.client.cancel().await?;
    Ok(())
}

#[tokio::test]
async fn outputs_preserve_sensitivity_and_report_unreadable_state() -> Result<()> {
    let session = Session::start(
        r#"
output "secret" {
  value = "never-return-this-secret"
  sensitive = true
}
output "public" { value = 42 }
"#,
        true,
    )
    .await?;
    session.value("init_terraform", json!({})).await?;
    let plan = session.value("get_terraform_plan", json!({})).await?;
    session
        .value(
            "apply_terraform",
            json!({"plan_id": plan["plan_id"], "auto_approve": true}),
        )
        .await?;
    for arguments in [json!({}), json!({"name": "secret"})] {
        let result = session.value("terraform_output", arguments).await?;
        assert!(!result.to_string().contains("never-return-this-secret"));
        let secret = result["outputs"]
            .as_array()
            .context("outputs array")?
            .iter()
            .find(|output| output["name"] == "secret")
            .context("secret output")?;
        assert_eq!(secret["sensitive"], true);
        assert_eq!(secret["value"], "[sensitive]");
    }
    let public = session
        .value("terraform_output", json!({"name": "public"}))
        .await?;
    assert_eq!(public["outputs"][0]["value"], 42);
    assert_eq!(public["outputs"][0]["sensitive"], false);
    assert_eq!(
        session
            .call("terraform_output", json!({"name": "missing"}))
            .await?
            .is_error,
        Some(true)
    );
    tokio::fs::write(
        session.directory.path().join("terraform.tfstate"),
        "unreadable state",
    )
    .await?;
    assert_eq!(
        session.call("terraform_output", json!({})).await?.is_error,
        Some(true)
    );
    for file in ["server.log", "audit.log"] {
        let log = tokio::fs::read_to_string(session.directory.path().join(file)).await?;
        assert!(!log.contains("never-return-this-secret"));
    }
    Ok(())
}

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
    assert_eq!(failed["status"], "failed");
    assert_eq!(failed["recovery"]["state_may_have_changed"], true);
    assert!(
        failed["recovery"]["next_steps"]
            .as_array()
            .is_some_and(|steps| !steps.is_empty())
    );
    let snapshot = session
        .value("get_terraform_plan", json!({"plan_id": second["plan_id"]}))
        .await?;
    assert_eq!(snapshot["status"], "failed");
    assert_eq!(snapshot["apply_result"], failed);
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

#[tokio::test]
async fn saved_plans_can_be_listed_and_discarded_without_affecting_other_plans() -> Result<()> {
    let session = Session::start(CONFIGURATION, true).await?;
    session.value("init_terraform", json!({})).await?;
    let first = session.value("get_terraform_plan", json!({})).await?;
    let second = session.value("get_terraform_plan", json!({})).await?;
    let listed = session.value("list_terraform_plans", json!({})).await?;
    let plans = listed["plans"].as_array().context("plans")?;
    assert_eq!(plans.len(), 2);
    assert_eq!(plans[0]["plan_id"], first["plan_id"]);
    assert_eq!(plans[1]["plan_id"], second["plan_id"]);
    assert_eq!(plans[0]["status"], "ready");
    assert_eq!(plans[0]["target"], first["target"]);
    assert!(
        plans[0].get("analysis").is_none(),
        "listing should contain metadata only"
    );
    let discarded = session
        .value(
            "discard_terraform_plan",
            json!({"plan_id": first["plan_id"]}),
        )
        .await?;
    assert_eq!(discarded["discarded"], true);
    assert!(!session.directory.path().join("terraform.tfstate").exists());
    for tool in [
        "get_terraform_plan",
        "apply_terraform",
        "discard_terraform_plan",
    ] {
        let result = session
            .call(
                tool,
                json!({"plan_id": first["plan_id"], "auto_approve": true}),
            )
            .await?;
        assert_eq!(
            result.is_error,
            Some(true),
            "{tool} must reject a discarded plan"
        );
    }
    let listed = session.value("list_terraform_plans", json!({})).await?;
    assert_eq!(
        listed["plans"].as_array().context("remaining plans")?.len(),
        1
    );
    session
        .value(
            "apply_terraform",
            json!({"plan_id": second["plan_id"], "auto_approve": true}),
        )
        .await?;
    session.client.cancel().await?;
    Ok(())
}

#[tokio::test]
async fn destroy_requires_a_saved_destroy_plan_and_preserves_delete_gates() -> Result<()> {
    let session = Session::start_with_options(CONFIGURATION, true, true, 900, false).await?;
    session.value("init_terraform", json!({})).await?;
    let create = session.value("get_terraform_plan", json!({})).await?;
    let rejected = session
        .call(
            "destroy_terraform",
            json!({"plan_id": create["plan_id"], "auto_approve": true}),
        )
        .await?;
    assert_eq!(rejected.is_error, Some(true));
    session
        .value(
            "apply_terraform",
            json!({"plan_id": create["plan_id"], "auto_approve": true}),
        )
        .await?;
    let before = tokio::fs::read(session.directory.path().join("terraform.tfstate")).await?;
    let plan = session
        .value("get_terraform_plan", json!({"destroy": true}))
        .await?;
    assert_eq!(plan["destroy"], true);
    let review = session
        .value("review_terraform_plan", json!({"plan_id": plan["plan_id"]}))
        .await?;
    assert_eq!(review["summary"], "0 add, 0 change, 1 destroy, 0 replace");
    assert_eq!(
        review["decision"], "block",
        "destructive risk assessment must be preserved"
    );
    for input in [
        json!({"auto_approve": true}),
        json!({"plan_id": plan["plan_id"]}),
    ] {
        assert_eq!(
            session.call("destroy_terraform", input).await?.is_error,
            Some(true)
        );
    }
    assert_eq!(
        before,
        tokio::fs::read(session.directory.path().join("terraform.tfstate")).await?
    );
    let applied = session
        .value(
            "destroy_terraform",
            json!({"plan_id": plan["plan_id"], "auto_approve": true}),
        )
        .await?;
    assert_eq!(applied["state_verified"], true);
    assert_eq!(applied["managed_resources"], 0);
    assert_eq!(applied["status"], "applied");
    assert_eq!(
        session
            .call(
                "destroy_terraform",
                json!({"plan_id": plan["plan_id"], "auto_approve": true})
            )
            .await?
            .is_error,
        Some(true)
    );
    session.client.cancel().await?;

    let denied = Session::start(CONFIGURATION, true).await?;
    denied.value("init_terraform", json!({})).await?;
    let plan = denied
        .value("get_terraform_plan", json!({"destroy": true}))
        .await?;
    for tool in ["apply_terraform", "destroy_terraform"] {
        let result = denied
            .call(
                tool,
                json!({"plan_id": plan["plan_id"], "auto_approve": true}),
            )
            .await?;
        assert_eq!(
            result.is_error,
            Some(true),
            "{tool} must enforce the delete gate"
        );
    }
    for input in [
        json!({"destroy": true, "refresh_only": true}),
        json!({"destroy": true, "replace": ["terraform_data.example"]}),
        json!({"destroy": true, "plan_id": plan["plan_id"]}),
    ] {
        assert_eq!(
            denied.call("get_terraform_plan", input).await?.is_error,
            Some(true)
        );
    }
    denied.client.cancel().await?;
    Ok(())
}

#[tokio::test]
async fn partial_apply_failure_preserves_state_and_explains_recovery() -> Result<()> {
    let session = Session::start(
        r#"resource "terraform_data" "completed" { input = "keep" }
resource "terraform_data" "failed" {
  depends_on = [terraform_data.completed]
  provisioner "local-exec" { command = "exit 1" }
}"#,
        true,
    )
    .await?;
    session.value("init_terraform", json!({})).await?;
    let plan = session.value("get_terraform_plan", json!({})).await?;
    let response = session
        .call(
            "apply_terraform",
            json!({"plan_id": plan["plan_id"], "auto_approve": true}),
        )
        .await?;
    assert_eq!(response.is_error, Some(true));
    let result = response.structured_content.context("failure result")?;
    assert_eq!(result["status"], "failed");
    assert_eq!(result["recovery"]["state_may_have_changed"], true);
    assert!(
        result["recovery"]["next_steps"]
            .as_array()
            .is_some_and(|steps| !steps.is_empty())
    );
    let state: Value = serde_json::from_slice(
        &tokio::fs::read(session.directory.path().join("terraform.tfstate")).await?,
    )?;
    assert!(
        state["resources"]
            .as_array()
            .context("resources")?
            .iter()
            .any(|resource| resource["name"] == "completed")
    );
    let saved = session
        .value("get_terraform_plan", json!({"plan_id": plan["plan_id"]}))
        .await?;
    assert_eq!(saved["apply_result"], result);
    let audit = session.audit_entries().await?;
    assert_eq!(audit.len(), 1);
    assert_eq!(audit[0].operation, "apply");
    assert!(!audit[0].success);
    assert!(
        audit[0]
            .error
            .as_ref()
            .is_some_and(|error| !error.is_empty())
    );
    assert_eq!(
        audit[0].command.last().map(String::as_str),
        plan["plan_id"].as_str()
    );
    session.client.cancel().await?;
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn timed_out_apply_retains_unknown_outcome_and_recovery_instructions() -> Result<()> {
    let session = Session::start_with_options(
        r#"resource "terraform_data" "slow" {
  provisioner "local-exec" { command = "echo started > started.txt; sleep 10" }
}"#,
        true,
        false,
        5,
        false,
    )
    .await?;
    session.value("init_terraform", json!({})).await?;
    let plan = session.value("get_terraform_plan", json!({})).await?;
    let result = session
        .call(
            "apply_terraform",
            json!({"plan_id": plan["plan_id"], "auto_approve": true}),
        )
        .await?;
    assert_eq!(result.is_error, Some(true));
    let result = result
        .structured_content
        .context("structured timeout result")?;
    assert_eq!(result["status"], "outcome_unknown");
    assert!(session.directory.path().join("started.txt").exists());
    assert!(result["exit_code"].is_null());
    assert_eq!(result["recovery"]["state_may_have_changed"], true);
    let snapshot = session
        .value("get_terraform_plan", json!({"plan_id": plan["plan_id"]}))
        .await?;
    assert_eq!(snapshot["status"], "outcome_unknown");
    assert_eq!(snapshot["apply_result"], result);
    let audit = session.audit_entries().await?;
    assert_eq!(audit.len(), 1);
    assert!(!audit[0].success);
    assert!(
        audit[0]
            .error
            .as_ref()
            .is_some_and(|error| error.contains("timed out"))
    );
    assert_eq!(
        session
            .call(
                "apply_terraform",
                json!({"plan_id": plan["plan_id"], "auto_approve": true})
            )
            .await?
            .is_error,
        Some(true)
    );
    session.client.cancel().await?;
    Ok(())
}

/// Explicit hands-on check: downloads a real provider and retains a local report.
#[tokio::test]
#[ignore = "requires the local_app example binary and provider registry access; see example/local-app/README.md"]
async fn local_web_app_is_deployed_updated_and_destroyed() -> Result<()> {
    use tokio::io::{AsyncBufReadExt, BufReader};

    let binary = std::env::var_os("TFMCP_LOCAL_APP_BINARY").context(
        "Build the local_app example and set TFMCP_LOCAL_APP_BINARY to its absolute path",
    )?;
    let session = Session::start_with_options(
        include_str!("../example/local-app/main.tf"),
        true,
        true,
        900,
        true,
    )
    .await?;
    let directory = session.directory.path();
    tokio::fs::write(
        directory.join("index.html.tftpl"),
        include_str!("../example/local-app/index.html.tftpl"),
    )
    .await?;
    let initialization = session.value("init_terraform", json!({})).await?;
    let formatting = session
        .value("terraform_fmt", json!({"check": true}))
        .await?;
    assert_eq!(formatting["success"], true);
    let validation = session.value("validate_terraform", json!({})).await?;
    let plan = session.value("get_terraform_plan", json!({})).await?;
    let review = session
        .value("review_terraform_plan", json!({"plan_id": plan["plan_id"]}))
        .await?;
    let initial_apply = session
        .value(
            "apply_terraform",
            json!({"plan_id": plan["plan_id"], "auto_approve": true}),
        )
        .await?;
    assert_eq!(initial_apply["state_verified"], true);

    let mut app = Command::new(binary)
        .arg("--directory")
        .arg(directory.join("site"))
        .env_clear()
        .env("PATH", std::env::var_os("PATH").context("test PATH")?)
        .envs(std::env::var_os("SystemRoot").map(|value| ("SystemRoot", value)))
        .env("HOME", directory.join("home"))
        .env("USERPROFILE", directory.join("home"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::from(std::fs::File::create(
            directory.join("app.log"),
        )?))
        .kill_on_drop(true)
        .spawn()?;
    let mut output = BufReader::new(app.stdout.take().context("app stdout")?).lines();
    let url = tokio::time::timeout(std::time::Duration::from_secs(10), output.next_line())
        .await??
        .context("app URL")?;
    let http = reqwest::Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;
    let response = http.get(&url).send().await?;
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let initial_html = response.text().await?;
    assert!(initial_html.contains("Local deployment · v1"));
    assert_eq!(
        http.get(format!("{url}/health"))
            .send()
            .await?
            .text()
            .await?,
        "ok"
    );

    tokio::fs::write(directory.join("release.tfvars"), "release = \"v2\"\n").await?;
    let update = session
        .value(
            "get_terraform_plan",
            json!({"var_files": ["release.tfvars"]}),
        )
        .await?;
    let update_review = session
        .value(
            "review_terraform_plan",
            json!({"plan_id": update["plan_id"]}),
        )
        .await?;
    // The reviewed v2 plan must win over a later, unreviewed edit to v3.
    tokio::fs::write(directory.join("release.tfvars"), "release = \"v3\"\n").await?;
    let updated_apply = session
        .value(
            "apply_terraform",
            json!({"plan_id": update["plan_id"], "auto_approve": true}),
        )
        .await?;
    assert_eq!(updated_apply["state_verified"], true);
    let updated_response = http.get(&url).send().await?;
    assert_eq!(updated_response.status(), reqwest::StatusCode::OK);
    let updated_html = updated_response.text().await?;
    assert!(updated_html.contains("Local deployment · v2"));
    assert!(!updated_html.contains("v3"));
    let release = session
        .value("terraform_output", json!({"name": "release"}))
        .await?;
    assert_eq!(release["outputs"][0]["value"], "v2");
    tokio::fs::create_dir(directory.join("preview")).await?;
    tokio::fs::write(directory.join("preview/index.html"), updated_html).await?;

    let teardown = session
        .value("get_terraform_plan", json!({"destroy": true}))
        .await?;
    let teardown_review = session
        .value(
            "review_terraform_plan",
            json!({"plan_id": teardown["plan_id"]}),
        )
        .await?;
    let destroyed = session
        .value(
            "destroy_terraform",
            json!({"plan_id": teardown["plan_id"], "auto_approve": true}),
        )
        .await?;
    assert_eq!(destroyed["managed_resources"], 0);
    assert_eq!(destroyed["state_verified"], true);
    assert!(!directory.join("site/index.html").exists());
    assert_eq!(
        http.get(&url).send().await?.status(),
        reqwest::StatusCode::NOT_FOUND
    );
    assert_eq!(
        http.get(format!("{url}/health"))
            .send()
            .await?
            .text()
            .await?,
        "ok"
    );

    let audit = session.audit_entries().await?;
    assert_eq!(audit.len(), 3);
    let terraform_log = tokio::fs::read_to_string(directory.join("terraform.log")).await?;
    let commands: Vec<_> = terraform_log
        .lines()
        .filter(|line| line.contains("CLI command args:"))
        .collect();
    let apply_commands: Vec<_> = commands
        .iter()
        .filter(|line| line.contains("{\"apply\","))
        .collect();
    assert_eq!(apply_commands.len(), 3);
    assert_eq!(
        commands
            .iter()
            .filter(|line| line.contains("{\"plan\","))
            .count(),
        3
    );
    assert!(commands.iter().any(|line| line.contains("\"-destroy\"")));
    for (entry, (operation, expected_plan)) in
        audit
            .iter()
            .zip([("apply", &plan), ("apply", &update), ("destroy", &teardown)])
    {
        assert_eq!(entry.operation, operation);
        assert!(entry.success);
        assert!(entry.error.is_none());
        assert_eq!(
            entry.command.last().map(String::as_str),
            expected_plan["plan_id"].as_str()
        );
        assert_eq!(
            std::path::Path::new(&entry.directory).canonicalize()?,
            directory.canonicalize()?
        );
    }
    for (entry, command) in audit.iter().zip(&apply_commands) {
        let plan_id = entry.command.last().context("audited plan ID")?;
        assert!(command.contains(&format!("/{plan_id}/plan.tfplan")));
        for option in ["-input=false", "-json", "-lock-timeout=30s"] {
            assert!(command.contains(option));
        }
    }
    let report = json!({
        "initialization": initialization, "formatting": formatting, "validation": validation,
        "initial_review": review, "initial_apply": initial_apply,
        "update_review": update_review, "updated_apply": updated_apply, "release_output": release,
        "destroy_review": teardown_review, "destroy_result": destroyed,
        "http_checks": {"initial_status": 200, "initial_release": "v1", "updated_status": 200, "updated_release": "v2", "after_destroy_status": 404, "health_after_destroy": "ok"},
        "audit_entries": audit,
        "terraform_apply_commands": apply_commands,
    });
    tokio::fs::write(
        directory.join("report.json"),
        serde_json::to_vec_pretty(&report)?,
    )
    .await?;
    app.kill().await?;
    app.wait().await?;
    session.client.cancel().await?;
    let artifacts = session.directory.keep();
    println!(
        "Local application verified; report, logs, lockfile, state and preview retained at {}",
        artifacts.display()
    );
    Ok(())
}
