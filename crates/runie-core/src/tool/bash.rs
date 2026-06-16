//! Bash tool — executes shell commands.

use crate::tool::{Tool, ToolContext, ToolOutput, ToolStatus};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::time::{Duration, Instant};

pub struct BashTool;

/// Default timeout for bash commands.
const DEFAULT_TIMEOUT_SECS: u64 = 60;

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        "Execute a shell command. Commands are subject to safety checks."
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Shell command to execute"
                },
                "timeout_seconds": {
                    "type": "integer",
                    "description": "Maximum execution time in seconds (default: 60)"
                }
            },
            "required": ["command"]
        })
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn requires_approval(&self, _input: &Value) -> bool {
        true
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolOutput> {
        let start = Instant::now();
        let command = input["command"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("command is required"))?;
        let timeout_secs = input["timeout_seconds"]
            .as_u64()
            .unwrap_or(DEFAULT_TIMEOUT_SECS);
        let timeout = Duration::from_secs(timeout_secs);

        // Run command in a blocking task
        let result = tokio::task::spawn_blocking({
            let command = command.to_string();
            let working_dir = ctx.working_dir.clone();
            move || run_bash_inner(&command, &working_dir, timeout)
        })
        .await
        .map_err(|e| anyhow::anyhow!("task join error: {}", e))?;

        Ok(ToolOutput {
            tool_name: "bash".to_string(),
            tool_args: serde_json::json!({ "command": command }),
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

fn run_bash_inner(command: &str, working_dir: &std::path::Path, timeout: Duration) -> BashResult {
    use std::process::Command;
    use std::sync::mpsc;
    use std::thread;

    let (tx, rx) = mpsc::channel();
    let work_dir = working_dir.to_path_buf();
    let cmd = command.to_string();

    thread::spawn(move || {
        let result = Command::new("bash")
            .args(["-c", &cmd])
            .current_dir(&work_dir)
            .output();
        let _ = tx.send(result);
    });

    match rx.recv_timeout(timeout) {
        Ok(Ok(output)) => process_output(output),
        Ok(Err(e)) => BashResult {
            output: format!("Error executing command: {}", e),
            bytes_transferred: None,
            status: ToolStatus::Error,
        },
        Err(mpsc::RecvTimeoutError::Timeout) => BashResult {
            output: format!(
                "Command timed out after {:.0} seconds",
                timeout.as_secs_f64()
            ),
            bytes_transferred: None,
            status: ToolStatus::TimedOut,
        },
        Err(mpsc::RecvTimeoutError::Disconnected) => BashResult {
            output: "Command channel disconnected unexpectedly".to_string(),
            bytes_transferred: None,
            status: ToolStatus::Error,
        },
    }
}

fn process_output(output: std::process::Output) -> BashResult {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = combine_output(&stdout, &stderr);
    let bytes = stdout.len() as u64 + stderr.len() as u64;
    let status = if output.status.success() {
        ToolStatus::Success
    } else {
        ToolStatus::Error
    };
    BashResult {
        output: combined,
        bytes_transferred: Some(bytes),
        status,
    }
}

fn combine_output(stdout: &str, stderr: &str) -> String {
    if stdout.is_empty() && stderr.is_empty() {
        return String::new();
    }
    if stdout.is_empty() {
        return stderr.trim_end().to_string();
    }
    if stderr.is_empty() {
        return stdout.trim_end().to_string();
    }
    format!("{}\n{}", stdout.trim_end(), stderr.trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bash_tool_runs_quick_command() {
        let result = run_bash_inner("echo hello", std::path::Path::new("."), Duration::from_secs(5));
        assert_eq!(result.status, ToolStatus::Success);
        assert!(result.output.contains("hello"));
    }

    #[test]
    fn bash_tool_respects_timeout_seconds() {
        let result = run_bash_inner(
            "sleep 10",
            std::path::Path::new("."),
            Duration::from_secs(1),
        );
        assert_eq!(result.status, ToolStatus::TimedOut);
        assert!(result.output.contains("timed out"));
    }
}
