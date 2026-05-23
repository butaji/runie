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

        let output = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            tokio::process::Command::new("sh")
                .arg("-c")
                .arg(command)
                .current_dir(&self.workspace.root)
                .kill_on_drop(true)
                .output()
        )
        .await
        .map_err(|e| ToolError::ExecutionFailed(format!("Command timed out: {}", e)))?
        .map_err(|e| ToolError::ExecutionFailed(format!("Command failed: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let content = if stderr.is_empty() {
            stdout
        } else {
            format!("{}\n[stderr]: {}", stdout, stderr)
        };

        // Clip output to reasonable size (e.g., 10k chars)
        let clipped = if content.len() > 10000 {
            format!("{}... [clipped, {} total chars]", &content[..10000], content.len())
        } else {
            content
        };

        Ok(ToolOutput {
            content: clipped,
            metadata: json!({"command": command, "exit_code": output.status.code()}),
            terminate: false,
        })
    }
}
