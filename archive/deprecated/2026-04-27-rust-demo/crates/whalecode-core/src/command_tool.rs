use std::{path::Path, time::Duration};

use serde::Deserialize;
use serde_json::json;
use tokio::{process::Command, time::timeout};

const DEFAULT_TIMEOUT_SECS: u64 = 60;
const MAX_TIMEOUT_SECS: u64 = 300;
const MAX_OUTPUT_BYTES: usize = 24 * 1024;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RunCommandArgs {
    pub(crate) command: String,
    pub(crate) args: Option<Vec<String>>,
    pub(crate) timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommandToolOutput {
    pub(crate) message: String,
    pub(crate) summary: String,
}

pub(crate) async fn run_command(workspace_root: &Path, args: RunCommandArgs) -> CommandToolOutput {
    let timeout_secs = args
        .timeout_secs
        .unwrap_or(DEFAULT_TIMEOUT_SECS)
        .clamp(1, MAX_TIMEOUT_SECS);
    let argv = args.args.unwrap_or_default();
    let mut command = Command::new(&args.command);
    command
        .args(&argv)
        .current_dir(workspace_root)
        .kill_on_drop(true);

    match timeout(Duration::from_secs(timeout_secs), command.output()).await {
        Ok(Ok(output)) => {
            let code = output.status.code();
            let stdout = truncate_lossy(&output.stdout);
            let stderr = truncate_lossy(&output.stderr);
            let ok = output.status.success();
            CommandToolOutput {
                message: serde_json::to_string(&json!({
                    "ok": ok,
                    "tool_status": if ok { "succeeded" } else { "failed" },
                    "command": args.command,
                    "args": argv,
                    "exit_code": code,
                    "stdout": stdout,
                    "stderr": stderr,
                    "timed_out": false,
                }))
                .expect("command result is serializable"),
                summary: match code {
                    Some(code) => format!("exit {code}"),
                    None => "terminated by signal".to_owned(),
                },
            }
        }
        Ok(Err(error)) => CommandToolOutput {
            message: serde_json::to_string(&json!({
                "ok": false,
                "tool_status": "failed",
                "command": args.command,
                "args": argv,
                "error": error.to_string(),
                "timed_out": false,
            }))
            .expect("command error is serializable"),
            summary: "command failed to start".to_owned(),
        },
        Err(_) => CommandToolOutput {
            message: serde_json::to_string(&json!({
                "ok": false,
                "tool_status": "failed",
                "command": args.command,
                "args": argv,
                "error": format!("command timed out after {timeout_secs}s"),
                "timed_out": true,
            }))
            .expect("command timeout is serializable"),
            summary: "command timed out".to_owned(),
        },
    }
}

fn truncate_lossy(bytes: &[u8]) -> String {
    let output = String::from_utf8_lossy(bytes);
    if output.len() <= MAX_OUTPUT_BYTES {
        return output.into_owned();
    }
    let mut boundary = MAX_OUTPUT_BYTES;
    while !output.is_char_boundary(boundary) {
        boundary -= 1;
    }
    format!("{}...[truncated]...", &output[..boundary])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn runs_command_in_workspace() {
        let workspace = tempfile::tempdir().expect("workspace");

        let output = run_command(
            workspace.path(),
            RunCommandArgs {
                command: "pwd".to_owned(),
                args: None,
                timeout_secs: Some(5),
            },
        )
        .await;

        assert!(output.message.contains("\"ok\":true"));
        assert!(output
            .message
            .contains(&workspace.path().display().to_string()));
        assert_eq!(output.summary, "exit 0");
    }
}
