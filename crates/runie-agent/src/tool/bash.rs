//! Bash tool — executes shell commands.

use crate::tool::{ToolContext, ToolOutput, ToolStatus};
use runie_core::bash_safety::check_bash_safety;
use runie_core::shell::{run_bash, run_bash_sandboxed, ShellStatus};
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

        // Check if sandbox is enabled via environment variable
        let use_sandbox = std::env::var("RUNIE_SANDBOX").as_deref() == Ok("1");

        let result = if use_sandbox {
            run_bash_sandboxed(
                &input.command,
                &ctx.working_dir,
                &ctx.env,
                timeout,
                true, // shell mode for bash tool
            )
            .await
        } else {
            run_bash(
                &input.command,
                &ctx.working_dir,
                &ctx.env,
                timeout,
                true, // shell mode for bash tool
            )
            .await
        };

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

    #[tokio::test]
    async fn bash_sandboxed_succeeds_when_enabled() {
        // Set sandbox environment variable
        std::env::set_var("RUNIE_SANDBOX", "1");

        // Ensure cleanup even if test panics
        struct SandboxGuard;
        impl Drop for SandboxGuard {
            fn drop(&mut self) {
                std::env::remove_var("RUNIE_SANDBOX");
            }
        }
        let _guard = SandboxGuard;

        let input = BashInput {
            command: "echo sandboxed".to_string(),
            timeout_seconds: Some(5),
        };
        let ctx = ToolContext::default();
        let output = BashTool::execute(input, &ctx).await;

        // Sandbox may not be available on all platforms, so we accept either success or error
        // (error could be "sandbox unavailable")
        match output.status {
            ToolStatus::Success => {}
            ToolStatus::Error => {
                // Sandbox not available on this platform - that's ok
                assert!(
                    output.content.contains("unavailable")
                        || output.content.contains("Sandbox")
                        || output.content.contains("sandboxed")
                );
            }
            _ => panic!("Unexpected status: {:?}", output.status),
        }
    }

    #[tokio::test]
    async fn bash_without_sandbox_env_works() {
        // Ensure sandbox env is not set
        std::env::remove_var("RUNIE_SANDBOX");

        let input = BashInput {
            command: "echo no_sandbox".to_string(),
            timeout_seconds: Some(5),
        };
        let ctx = ToolContext::default();
        let output = BashTool::execute(input, &ctx).await;
        assert_eq!(output.status, ToolStatus::Success);
        assert!(output.content.contains("no_sandbox"));
    }
}
