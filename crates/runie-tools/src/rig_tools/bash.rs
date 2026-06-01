//! Bash tool implementation for rig-core Tool trait.

use std::path::PathBuf;
use std::time::Duration;

use rig_core::completion::ToolDefinition;
use rig_core::tool::Tool;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Allowlist of safe commands that can be executed without explicit confirmation.
const SAFE_COMMANDS: &[&str] = &[
    "echo", "cat", "ls", "pwd", "find", "grep", "head", "tail", "wc", "mkdir", "touch", "cp",
    "mv", "git", "cargo", "rustc", "make", "npm", "node", "python", "python3",
];

/// Commands that are allowed but with additional restrictions.
const RESTRICTED_COMMANDS: &[&str] = &["rm"];

/// Secondary blacklist of dangerous patterns that should always be blocked.
const DANGEROUS_PATTERNS: &[&str] = &[
    "rm -rf /", "rm -rf /*", "rm -rf ~/", ":(){ :|:& };:", ": () { : | : & } ; :", "> /dev/sda",
    "/proc/", "/sys/", "--no-preserve-root",
];

/// Secondary blacklist of shell metacharacters that enable injection.
const FORBIDDEN_CHARS: &[char] = &['|', ';', '&', '>', '<', '(', ')', '`', '$'];

#[derive(Debug, Deserialize)]
pub struct BashArgs {
    pub command: String,
    #[serde(default = "default_timeout")]
    pub timeout: Option<u64>,
}

fn default_timeout() -> Option<u64> {
    Some(60)
}

#[derive(Debug, Serialize)]
pub struct BashOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clipped: Option<String>,
}

#[derive(Debug, Error)]
pub enum BashError {
    #[error("command execution failed: {0}")]
    ExecutionFailed(String),
    #[error("command timed out after {0}s")]
    Timeout(u64),
    #[error("command blocked by allowlist")]
    Blocked,
    #[error("invalid arguments: {0}")]
    InvalidArguments(String),
}

pub struct BashTool {
    workspace: PathBuf,
}

impl BashTool {

    #[must_use]
    #[must_use]
    pub fn new(workspace: PathBuf) -> Self {
        Self { workspace }
    }

    fn extract_base_command(command: &str) -> &str {
        command.trim().split_whitespace().next().unwrap_or("")
    }

    fn is_allowed_command(base_cmd: &str) -> bool {
        SAFE_COMMANDS.contains(&base_cmd)
    }

    fn is_restricted_command_safe(base_cmd: &str, command: &str) -> Result<bool, BashError> {
        match base_cmd {
            "rm" => {
                let dangerous_rm_patterns = ["-rf", "-r -f", "-f -r", "/", "/*", "~/"];
                for pattern in &dangerous_rm_patterns {
                    if command.contains(pattern) {
                        return Err(BashError::ExecutionFailed(format!(
                            "Restricted command 'rm' used with dangerous flag/path: '{}'. \
                             Use 'rm' only for simple file deletion without -rf flags.",
                            pattern
                        )));
                    }
                }
                Ok(true)
            }
            _ => Ok(true),
        }
    }

    fn validate_command(command: &str) -> Result<(), BashError> {
        let base_cmd = Self::extract_base_command(command);

        if Self::is_allowed_command(base_cmd) {
            return Ok(());
        }

        if RESTRICTED_COMMANDS.contains(&base_cmd) {
            if Self::is_restricted_command_safe(base_cmd, command)? {
                return Ok(());
            }
        }

        Err(BashError::Blocked)
    }

    fn check_dangerous_patterns(command: &str) -> Result<(), BashError> {
        for pattern in DANGEROUS_PATTERNS {
            if command.contains(pattern) {
                return Err(BashError::ExecutionFailed(format!(
                    "Dangerous command pattern '{}' blocked",
                    pattern
                )));
            }
        }
        Ok(())
    }

    fn check_forbidden_chars(command: &str) -> Result<(), BashError> {
        for ch in FORBIDDEN_CHARS {
            if command.contains(*ch) {
                let is_allowed = match ch {
                    '&' => command.contains("&&") || command.contains(">&"),
                    '>' => command.contains(">&"),
                    _ => false,
                };
                if !is_allowed {
                    return Err(BashError::ExecutionFailed(format!(
                        "Shell metacharacter '{}' detected. Use simple commands only.",
                        ch
                    )));
                }
            }
        }
        Ok(())
    }

    /// Formats raw command output into clipped content.
    fn format_output(stdout: &str, stderr: &str) -> (Option<String>, String) {
        let content = if stderr.is_empty() {
            stdout.to_string()
        } else {
            format!("{}\n[stderr]: {}", stdout, stderr)
        };
        if content.len() > 10000 {
            (
                Some(format!("[clipped, {} total chars]", content.len())),
                format!("{}... [clipped]", &content[..10000]),
            )
        } else {
            (None, content)
        }
    }

    /// Validates and executes a bash command, returning structured output.
    fn run_command(workspace: &PathBuf, command: &str, timeout: u64) -> Result<BashOutput, BashError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| BashError::ExecutionFailed(format!("Failed to create tokio runtime: {}", e)))?;
        let output = runtime.block_on(async {
                tokio::time::timeout(
                    Duration::from_secs(timeout),
                    tokio::process::Command::new("sh")
                        .arg("-c")
                        .arg(command)
                        .current_dir(workspace)
                        .kill_on_drop(true)
                        .output(),
                )
                .await
            })
            .map_err(|_| BashError::Timeout(timeout))?
            .map_err(|e| BashError::ExecutionFailed(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let (clipped, final_content) = Self::format_output(&stdout, &stderr);

        Ok(BashOutput {
            stdout: final_content,
            stderr,
            exit_code: output.status.code().unwrap_or(-1),
            clipped,
        })
    }
}

impl Tool for BashTool {
    const NAME: &'static str = "bash";

    type Error = BashError;
    type Args = BashArgs;
    type Output = BashOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Execute a bash command in the workspace directory".to_string(),
            parameters: serde_json::json!({
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

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let timeout = args.timeout.unwrap_or(60);

        // Primary security check: whitelist approach
        Self::validate_command(&args.command)?;

        // Secondary defense: check for dangerous patterns
        Self::check_dangerous_patterns(&args.command)?;

        // Secondary defense: check for shell metacharacters
        Self::check_forbidden_chars(&args.command)?;

        Self::run_command(&self.workspace, &args.command, timeout)
    }
}
