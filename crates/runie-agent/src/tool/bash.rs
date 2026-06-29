//! Bash tool — executes shell commands.

use crate::tool::{ToolContext, ToolOutput, ToolStatus};
use runie_core::bash_safety::check_bash_safety;
use runie_core::tool::ToolDef;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::time::{Duration, Instant};

/// Input parameters for bash tool.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct BashInput {
    /// Shell command to execute
    pub command: String,
    /// Maximum execution time in seconds (default: 60)
    #[serde(default)]
    pub timeout_seconds: Option<u64>,
}

pub struct BashTool;

/// Default timeout for bash commands.
const DEFAULT_TIMEOUT_SECS: u64 = 60;

impl ToolDef for BashTool {
    type Input = BashInput;

    const NAME: &'static str = "bash";
    const DESCRIPTION: &'static str =
        "Execute a shell command. Commands are subject to safety checks.";
    const READ_ONLY: bool = false;
    const REQUIRES_APPROVAL: bool = true;

    async fn execute(input: Self::Input, ctx: &ToolContext) -> ToolOutput {
        let start = Instant::now();
        let tool_args = serde_json::json!({ "command": input.command });

        if let Some(reason) = check_bash_safety(&input.command) {
            return ToolOutput::blocked("bash", tool_args, reason.to_owned());
        }
        let timeout_secs = input.timeout_seconds.unwrap_or(DEFAULT_TIMEOUT_SECS);
        let timeout = Duration::from_secs(timeout_secs);

        let result = run_bash_inner(&input.command, &ctx.working_dir, &ctx.env, timeout).await;

        ToolOutput {
            tool_name: "bash".to_owned(),
            tool_args,
            content: result.output,
            bytes_transferred: result.bytes_transferred,
            duration: start.elapsed(),
            status: result.status,
        }
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
        let input = BashInput {
            command: "sleep 30".to_string(),
            timeout_seconds: Some(1),
        };
        let ctx = ToolContext::default();
        let output = BashTool::execute(input, &ctx).await;
        assert_eq!(output.status, ToolStatus::TimedOut);
        assert!(output.content.contains("timed out"));
    }

    #[tokio::test]
    async fn bash_command_succeeds() {
        let input = BashInput {
            command: "echo hello".to_string(),
            timeout_seconds: Some(5),
        };
        let ctx = ToolContext::default();
        let output = BashTool::execute(input, &ctx).await;
        assert_eq!(output.status, ToolStatus::Success);
        assert!(output.content.contains("hello"));
    }

    #[tokio::test]
    async fn bash_command_fails() {
        let input = BashInput {
            command: "exit 1".to_string(),
            timeout_seconds: Some(5),
        };
        let ctx = ToolContext::default();
        let output = BashTool::execute(input, &ctx).await;
        assert_eq!(output.status, ToolStatus::Error);
    }
}
