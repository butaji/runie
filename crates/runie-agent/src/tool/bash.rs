//! Bash tool — executes shell commands.

use crate::define_tool;
use crate::tool::{Tool, ToolContext, ToolOutput, ToolStatus};
use anyhow::Result;
use async_trait::async_trait;
use runie_core::bash_safety::check_bash_safety;
use serde_json::Value;
use std::time::{Duration, Instant};

pub struct BashTool;

/// Default timeout for bash commands.
const DEFAULT_TIMEOUT_SECS: u64 = 60;

#[allow(clippy::use_self)]
#[async_trait]
impl Tool for BashTool {
    define_tool! {
        name: "bash",
        description: "Execute a shell command. Commands are subject to safety checks.",
        read_only: false,
        approval: true,
        fields: {
            "command": ("string", "Shell command to execute"),
            "timeout_seconds": ("integer", "Maximum execution time in seconds (default: 60)")
        },
        required: ["command"]
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolOutput> {
        let start = Instant::now();
        let command = input["command"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("command is required"))?;
        let tool_args = serde_json::json!({ "command": command });

        if let Some(reason) = check_bash_safety(command) {
            return Ok(ToolOutput::blocked("bash", tool_args, reason.to_owned()));
        }
        let timeout_secs = input["timeout_seconds"]
            .as_u64()
            .unwrap_or(DEFAULT_TIMEOUT_SECS);
        let timeout = Duration::from_secs(timeout_secs);

        let result = run_bash_inner(command, &ctx.working_dir, &ctx.env, timeout).await;

        Ok(ToolOutput {
            tool_name: "bash".to_owned(),
            tool_args,
            content: result.output,
            bytes_transferred: result.bytes_transferred,
            duration: start.elapsed(),
            status: result.status,
        })
    }
}

struct BashResult {
    output: String,
    bytes_transferred: Option<u64>,
    status: ToolStatus,
}

async fn run_bash_inner(
    command: &str,
    working_dir: &std::path::Path,
    env: &std::collections::HashMap<String, String>,
    timeout: Duration,
) -> BashResult {
    let mut cmd = tokio::process::Command::new("bash");
    cmd.arg("-c")
        .arg(command)
        .current_dir(working_dir)
        .envs(env)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return bash_error(&format!("Failed to spawn command: {}", e)),
    };

    match tokio::time::timeout(timeout, child.wait()).await {
        Ok(Ok(status)) => {
            let stdout = child.stdout.take();
            let stderr = child.stderr.take();
            collect_output(status, stdout, stderr).await
        }
        Ok(Err(e)) => bash_error(&format!("Error waiting for command: {}", e)),
        Err(_) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            bash_timeout(timeout)
        }
    }
}

async fn collect_output(
    status: std::process::ExitStatus,
    stdout: Option<tokio::process::ChildStdout>,
    stderr: Option<tokio::process::ChildStderr>,
) -> BashResult {
    use tokio::io::AsyncReadExt;

    let mut stdout_buf = String::new();
    let mut stderr_buf = String::new();

    if let Some(mut s) = stdout {
        let _ = s.read_to_string(&mut stdout_buf).await;
    }
    if let Some(mut s) = stderr {
        let _ = s.read_to_string(&mut stderr_buf).await;
    }

    let combined = combine_output(&stdout_buf, &stderr_buf);
    let bytes = stdout_buf.len() as u64 + stderr_buf.len() as u64;
    let tool_status = if status.success() {
        ToolStatus::Success
    } else {
        ToolStatus::Error
    };

    BashResult {
        output: combined,
        bytes_transferred: Some(bytes),
        status: tool_status,
    }
}

fn bash_error(msg: &str) -> BashResult {
    BashResult {
        output: msg.to_owned(),
        bytes_transferred: None,
        status: ToolStatus::Error,
    }
}

fn bash_timeout(timeout: Duration) -> BashResult {
    BashResult {
        output: format!(
            "Command timed out after {:.0} seconds",
            timeout.as_secs_f64()
        ),
        bytes_transferred: None,
        status: ToolStatus::TimedOut,
    }
}

fn combine_output(stdout: &str, stderr: &str) -> String {
    if stdout.is_empty() && stderr.is_empty() {
        return String::new();
    }
    if stdout.is_empty() {
        return stderr.trim_end().to_owned();
    }
    if stderr.is_empty() {
        return stdout.trim_end().to_owned();
    }
    format!("{}\n{}", stdout.trim_end(), stderr.trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combine_output_prefers_nonempty_streams() {
        assert!(combine_output("", "").is_empty());
        assert_eq!(combine_output("out", ""), "out");
        assert_eq!(combine_output("", "err"), "err");
        assert_eq!(combine_output("out", "err"), "out\nerr");
    }

    #[tokio::test]
    async fn bash_timeout_kills_child() {
        let tool = BashTool;
        let input = serde_json::json!({
            "command": "sleep 30",
            "timeout_seconds": 1,
        });
        let ctx = ToolContext::default();
        let output = tool.call(input, &ctx).await.unwrap();
        assert_eq!(output.status, ToolStatus::TimedOut);
        assert!(output.content.contains("timed out"));
    }

    #[tokio::test]
    async fn bash_command_succeeds() {
        let tool = BashTool;
        let input = serde_json::json!({
            "command": "echo hello",
            "timeout_seconds": 5,
        });
        let ctx = ToolContext::default();
        let output = tool.call(input, &ctx).await.unwrap();
        assert_eq!(output.status, ToolStatus::Success);
        assert!(output.content.contains("hello"));
    }

    #[tokio::test]
    async fn bash_command_fails() {
        let tool = BashTool;
        let input = serde_json::json!({
            "command": "exit 1",
            "timeout_seconds": 5,
        });
        let ctx = ToolContext::default();
        let output = tool.call(input, &ctx).await.unwrap();
        assert_eq!(output.status, ToolStatus::Error);
    }
}
