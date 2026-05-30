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

/// Allowlist of safe commands that can be executed without explicit confirmation.
/// Commands like `rm` have additional restrictions applied.
const SAFE_COMMANDS: &[&str] = &[
    "echo",
    "cat",
    "ls",
    "pwd",
    "find",
    "grep",
    "head",
    "tail",
    "wc",
    "mkdir",
    "touch",
    "cp",
    "mv",
    "git",
    "cargo",
    "rustc",
    "make",
    "npm",
    "node",
    "python",
    "python3",
    "sleep",
    "date",
    "seq",
    "printf",
    "exit",
];

/// Commands that are allowed but with additional restrictions.
const RESTRICTED_COMMANDS: &[&str] = &["rm"];

/// Secondary blacklist of dangerous patterns that should always be blocked.
const DANGEROUS_PATTERNS: &[&str] = &[
    "rm -rf /",
    "rm -rf /*",
    "rm -rf ~/",
    ":(){ :|:& };:",
    ": () { : | : & } ; :",
    "> /dev/sda",
    "/proc/",
    "/sys/",
    "--no-preserve-root",
];

/// Secondary blacklist of shell metacharacters that enable injection.
const FORBIDDEN_CHARS: &[char] = &['|', ';', '&', '>', '<', '(', ')', '`', '$'];

/// Patterns that spawn subshells - blocked even for whitelisted commands.
const SUBSHELL_PATTERNS: &[&str] = &[
    "$(",  // Command substitution
    "|",    // Pipe
    "&&",   // And chain
    "||",   // Or chain
    ";",    // Sequential
    "bash -c",  // Explicit bash subshell
    "sh -c",    // Explicit sh subshell
    "zsh -c",   // Explicit zsh subshell
    "ksh -c",   // Explicit ksh subshell
];

impl BashTool {
    /// Extracts the base command (first whitespace-delimited token) from a command string.
    fn extract_base_command(command: &str) -> &str {
        command.trim().split_whitespace().next().unwrap_or("")
    }

    /// Checks if a command is in the allowlist.
    fn is_allowed_command(base_cmd: &str) -> bool {
        SAFE_COMMANDS.contains(&base_cmd)
    }

    /// Checks if a restricted command has safe arguments.
    fn is_restricted_command_safe(base_cmd: &str, command: &str) -> Result<bool, ToolError> {
        match base_cmd {
            "rm" => {
                // Block recursive deletion, force deletion, and dangerous paths
                let dangerous_rm_patterns = [
                    "-rf", "-r -f", "-f -r",
                    "/", "/*", "~/",
                ];
                for pattern in &dangerous_rm_patterns {
                    if command.contains(pattern) {
                        return Err(ToolError::ExecutionFailed(format!(
                            "Restricted command 'rm' used with dangerous flag/path: '{}'. \
                             Use 'rm' only for simple file deletion without -rf flags.", pattern
                        )));
                    }
                }
                Ok(true)
            }
            _ => Ok(true),
        }
    }

    /// Validates a command against the security rules.
    fn validate_command(command: &str) -> Result<(), ToolError> {
        let base_cmd = Self::extract_base_command(command);

        // Check if command is in allowlist
        if Self::is_allowed_command(base_cmd) {
            return Ok(());
        }

        // Check if command is restricted with additional validation
        if RESTRICTED_COMMANDS.contains(&base_cmd) {
            if Self::is_restricted_command_safe(base_cmd, command)? {
                return Ok(());
            }
        }

        // Command not in allowlist - reject
        return Err(ToolError::ExecutionFailed(format!(
            "Command '{}' is not in the allowlist. Allowed commands: {}. \
             Use only whitelisted commands for security.",
            base_cmd,
            SAFE_COMMANDS.join(", ")
        )));
    }

    /// Secondary defense: check for dangerous patterns.
    fn check_dangerous_patterns(command: &str) -> Result<(), ToolError> {
        for pattern in DANGEROUS_PATTERNS {
            if command.contains(pattern) {
                return Err(ToolError::ExecutionFailed(format!(
                    "Dangerous command pattern '{}' blocked", pattern
                )));
            }
        }
        Ok(())
    }

    /// Secondary defense: check for shell metacharacters that enable injection.
    fn check_forbidden_chars(command: &str) -> Result<(), ToolError> {
        for ch in FORBIDDEN_CHARS {
            if command.contains(*ch) {
                // Allow specific safe patterns
                let is_allowed = match ch {
                    '&' => command.contains("&&") || command.contains(">&"),
                    '>' => command.contains(">&"),
                    _ => false,
                };
                if !is_allowed {
                    return Err(ToolError::ExecutionFailed(format!(
                        "Shell metacharacter '{}' detected. Use simple commands only.", ch
                    )));
                }
            }
        }
        Ok(())
    }

    /// Check for subshell spawning patterns - blocks even whitelisted commands from
    /// spawning subshells to prevent allowlist bypass (e.g., "echo $(malicious)").
    fn check_subshell_patterns(command: &str) -> Result<(), ToolError> {
        for pattern in SUBSHELL_PATTERNS {
            if command.contains(pattern) {
                return Err(ToolError::ExecutionFailed(format!(
                    "Subshell pattern '{}' blocked. Even whitelisted commands cannot spawn subshells.", pattern
                )));
            }
        }
        Ok(())
    }

    /// Formats command output into ToolOutput content.
    fn format_output(stdout: &str, stderr: &str, _exit_code: Option<i32>) -> String {
        let content = if stderr.is_empty() {
            stdout.to_string()
        } else {
            format!("{}\n[stderr]: {}", stdout, stderr)
        };

        // Clip output to reasonable size (e.g., 10k chars)
        if content.len() > 10000 {
            format!("{}... [clipped, {} total chars]", &content[..10000], content.len())
        } else {
            content
        }
    }

    /// Executes a validated command and returns the output.
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
        let content = Self::format_output(&stdout, &stderr, output.status.code());

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
        "Execute a bash command in the workspace directory. Use with caution."
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
                        "description": "The bash command to execute"
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
        let command = args["command"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'command' argument".to_string()))?;
        let timeout_secs = args["timeout"].as_u64().unwrap_or(60);

        // Primary security check: whitelist approach
        BashTool::validate_command(command)?;

        // Secondary defense: check for dangerous patterns
        BashTool::check_dangerous_patterns(command)?;

        // Secondary defense: check for shell metacharacters
        BashTool::check_forbidden_chars(command)?;

        // Tertiary defense: prevent subshell spawning even for whitelisted commands
        BashTool::check_subshell_patterns(command)?;

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
        // Empty string passes the .ok_or check but fails validation
        assert!(result.is_err());
        let err = result.unwrap_err();
        // Should fail because empty command is not in allowlist
        assert!(err.to_string().contains("not in the allowlist") || err.to_string().contains("Missing"));
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
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("not in the allowlist"));
    }
}
