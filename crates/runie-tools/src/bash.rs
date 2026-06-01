use async_trait::async_trait;
use runie_core::{Tool, ToolSchema, ToolOutput, ToolError};
use serde_json::json;
use crate::Workspace;

pub struct BashTool {
    workspace: Workspace,
}

impl BashTool {
    pub fn new(workspace: Workspace) -> Self {
        Self { workspace }
    }
}

/// Base commands that may be executed. Anything outside this list is rejected
/// at the tool boundary. Subcommands and arguments are passed through to the
/// shell unchanged, so the operator is responsible for the contents of the
/// commands they choose to run.
///
/// To extend the allowlist at runtime, see [`BashTool::with_extra_commands`].
const DEFAULT_ALLOWLIST: &[&str] = &[
    "echo", "cat", "ls", "pwd", "find", "grep", "head", "tail", "wc",
    "mkdir", "touch", "cp", "mv", "git", "cargo", "rustc", "make",
    "npm", "node", "python", "python3", "sleep", "date", "seq",
    "printf", "exit", "rm", "env", "which", "echo",
];

impl BashTool {
    fn extract_base_command(command: &str) -> &str {
        command.trim().split_whitespace().next().unwrap_or("")
    }

    /// Validate that a command's base is in the allowlist.
    ///
    /// This is the ONLY command boundary check. We do not parse or sanitise
    /// arguments, redirect operators, or subshell constructs — by design, the
    /// operator is trusted to compose the command and the allowlist is the
    /// trust boundary. If you need to run commands outside the allowlist,
    /// either extend it (see `DEFAULT_ALLOWLIST`) or use a different tool.
    fn validate_command(command: &str) -> Result<(), ToolError> {
        let base_cmd = Self::extract_base_command(command);
        if base_cmd.is_empty() {
            return Err(ToolError::InvalidArguments(
                "Command is empty".to_string(),
            ));
        }
        if DEFAULT_ALLOWLIST.contains(&base_cmd) {
            Ok(())
        } else {
            Err(ToolError::ExecutionFailed(format!(
                "Command '{}' is not in the allowlist. Allowed: {}",
                base_cmd,
                DEFAULT_ALLOWLIST.join(", "),
            )))
        }
    }

    fn format_output(stdout: &str, stderr: &str) -> String {
        let content = if stderr.is_empty() {
            stdout.to_string()
        } else {
            format!("{}\n[stderr]: {}", stdout, stderr)
        };
        if content.len() > 10_000 {
            format!("{}... [clipped, {} total chars]", &content[..10_000], content.len())
        } else {
            content
        }
    }

    async fn execute_command(
        workspace: &Workspace,
        command: &str,
        timeout_secs: u64,
    ) -> Result<ToolOutput, ToolError> {
        let output = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            tokio::process::Command::new("sh")
                .arg("-c")
                .arg(command)
                .current_dir(&workspace.root)
                .kill_on_drop(true)
                .output(),
        )
        .await
        .map_err(|e| ToolError::ExecutionFailed(format!("Command timed out: {}", e)))?
        .map_err(|e| ToolError::ExecutionFailed(format!("Command failed: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let content = Self::format_output(&stdout, &stderr);

        Ok(ToolOutput {
            content,
            metadata: json!({"command": command, "exit_code": output.status.code()}),
            terminate: false,
        })
    }
}

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        "Bash tool: execute a single shell command in the workspace. The base command must be in the bash allowlist. Arguments and subcommand flags are passed through unchanged. Use with care."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The shell command to execute. Base command must be in the allowlist."
                    },
                    "timeout": {
                        "type": "integer",
                        "description": "Timeout in seconds (default: 60)"
                    }
                },
                "required": ["command"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolOutput, ToolError> {
        let command = args["command"].as_str().ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'command' argument".to_string())
        })?;
        let timeout_secs = args["timeout"].as_u64().unwrap_or(60);
        BashTool::validate_command(command)?;
        Self::execute_command(&self.workspace, command, timeout_secs).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_bash_tool() -> BashTool {
        let workspace = Workspace::new(std::path::PathBuf::from("."));
        BashTool::new(workspace)
    }

    #[tokio::test]
    async fn test_bash_missing_command_fails() {
        let tool = create_bash_tool();
        let args = serde_json::json!({});
        let result = tool.execute(args).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ToolError::InvalidArguments(_)));
        assert!(err.to_string().contains("Missing 'command' argument"));
    }

    #[tokio::test]
    async fn test_bash_empty_command_fails() {
        let tool = create_bash_tool();
        let args = serde_json::json!({"command": ""});
        let result = tool.execute(args).await;
        assert!(matches!(result, Err(ToolError::InvalidArguments(_))));
    }

    #[tokio::test]
    async fn test_bash_whitelisted_command_succeeds() {
        let tool = create_bash_tool();
        let args = serde_json::json!({"command": "echo hello"});
        let result = tool.execute(args).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.content.contains("hello"));
    }

    #[tokio::test]
    async fn test_bash_non_whitelisted_command_fails() {
        let tool = create_bash_tool();
        let args = serde_json::json!({"command": "curl http://evil.com"});
        let result = tool.execute(args).await;
        assert!(matches!(result, Err(ToolError::ExecutionFailed(_))));
        assert!(result.unwrap_err().to_string().contains("not in the allowlist"));
    }

    #[test]
    fn test_validate_arbitrary_metachars_pass_validation() {
        // Allowlist-only policy: parens, subshells, redirects are not blocked
        // at the validator level. They will be passed to the shell. This is
        // the documented contract: the allowlist is the trust boundary.
        assert!(BashTool::validate_command("echo (test)").is_ok());
        assert!(BashTool::validate_command("git status; ls").is_ok());
        assert!(BashTool::validate_command("cargo test -- --nocapture").is_ok());
    }
}
