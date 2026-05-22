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

        // Block dangerous commands
        let dangerous = ["rm -rf /", "rm -rf /*", ":(){ :|:& };:", "> /dev/sda"];
        for pattern in &dangerous {
            if command.contains(pattern) {
                return Err(ToolError::ExecutionFailed(
                    format!("Blocked dangerous command: {}", pattern)
                ));
            }
        }

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
