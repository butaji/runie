//! Bash tool — executes shell commands.

use crate::tool::{ToolContext, ToolOutput, ToolStatus};
use runie_core::bash_safety::check_bash_safety;
use runie_core::shell::{run_bash, ShellStatus};
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

        let result = run_bash(
            &input.command,
            &ctx.working_dir,
            &ctx.env,
            timeout,
            true, // shell mode for bash tool
        )
        .await;

        let status = match result.status {
            ShellStatus::Success => ToolStatus::Success,
            ShellStatus::Error => ToolStatus::Error,
            ShellStatus::TimedOut => ToolStatus::TimedOut,
        };

        ToolOutput {
            tool_name: "bash".to_owned(),
            tool_args,
            content: result.output,
            bytes_transferred: result.bytes_transferred,
            duration: start.elapsed(),
            status,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
