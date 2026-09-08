//! Non-interactive, bounded Terraform subprocess execution.
use anyhow::{Context, Result};
use std::path::Path;
use std::process::{Output, Stdio};
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

const MAX_OUTPUT_BYTES: u64 = 32 * 1024 * 1024;

pub(super) async fn run(executable: &Path, directory: &Path, args: &[&str]) -> Result<Output> {
    let seconds = std::env::var("TFMCP_COMMAND_TIMEOUT_SECONDS")
        .map(|value| value.parse::<u64>())
        .unwrap_or(Ok(900))
        .context("TFMCP_COMMAND_TIMEOUT_SECONDS must be a positive integer")?;
    anyhow::ensure!(
        seconds > 0,
        "TFMCP_COMMAND_TIMEOUT_SECONDS must be positive"
    );
    run_with_timeout(executable, directory, args, Duration::from_secs(seconds)).await
}

async fn run_with_timeout(
    executable: &Path,
    directory: &Path,
    args: &[&str],
    timeout: Duration,
) -> Result<Output> {
    let mut child = Command::new(executable)
        .args(args)
        .current_dir(directory)
        .env("TF_INPUT", "0")
        .env("TF_IN_AUTOMATION", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .context("Could not start Terraform")?;
    let stdout = child
        .stdout
        .take()
        .context("Terraform stdout unavailable")?;
    let stderr = child
        .stderr
        .take()
        .context("Terraform stderr unavailable")?;
    let collect = async {
        let (stdout, stderr, status) =
            tokio::try_join!(read_output(stdout), read_output(stderr), async {
                child.wait().await.map_err(anyhow::Error::from)
            })?;
        Ok::<_, anyhow::Error>(Output {
            status,
            stdout,
            stderr,
        })
    };
    let result = tokio::time::timeout(timeout, collect).await
        .with_context(|| format!("Terraform timed out after {} seconds. The process was stopped; inspect state before retrying a write operation", timeout.as_secs()))
        .and_then(std::convert::identity);
    if result.is_err() {
        let _ = child.kill().await;
        let _ = child.wait().await;
    }
    result
}

async fn read_output(stream: impl tokio::io::AsyncRead + Unpin) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    stream
        .take(MAX_OUTPUT_BYTES + 1)
        .read_to_end(&mut bytes)
        .await?;
    anyhow::ensure!(
        bytes.len() as u64 <= MAX_OUTPUT_BYTES,
        "Terraform output exceeds 32 MiB; narrow the operation and inspect local logs"
    );
    Ok(bytes)
}

/// Diagnostic summaries omit source snippets and values from Terraform output.
pub(super) fn diagnostics(output: &Output) -> Vec<String> {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut diagnostics = Vec::new();
    for line in stdout.lines() {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(line)
            && let Some(summary) = value["diagnostic"]["summary"].as_str()
        {
            diagnostics.push(summary.to_string());
        }
    }
    if diagnostics.is_empty()
        && let Ok(value) = serde_json::from_str::<serde_json::Value>(&stdout)
        && let Some(items) = value["diagnostics"].as_array()
    {
        diagnostics.extend(
            items
                .iter()
                .filter_map(|item| item["summary"].as_str().map(str::to_owned)),
        );
    }
    if diagnostics.is_empty() && !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        diagnostics.push(
            if stderr.contains("No state") || stderr.contains("no state") {
                "No state file was found".to_string()
            } else {
                "Terraform failed; check initialization, backend credentials, and local diagnostics"
                    .to_string()
            },
        );
    }
    diagnostics
}

pub(super) fn ensure_success(output: &Output, operation: &str) -> Result<()> {
    anyhow::ensure!(
        output.status.success(),
        "Terraform {operation} failed (exit {:?}): {}",
        output.status.code(),
        diagnostics(output).join("; ")
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn subprocess_timeout_is_an_error() -> Result<()> {
        let executable = which::which("terraform")?;
        let directory = tempfile::tempdir()?;
        let result = run_with_timeout(
            &executable,
            directory.path(),
            &["version", "-json"],
            Duration::from_nanos(1),
        )
        .await;
        assert!(result.is_err());
        Ok(())
    }
}
