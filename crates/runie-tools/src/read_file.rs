use async_trait::async_trait;
use runie_core::{Tool, ToolSchema, ToolOutput, ToolError};
use serde_json::json;
use crate::Workspace;

pub struct ReadFileTool {
    workspace: Workspace,
}

impl ReadFileTool {
    pub fn new(workspace: Workspace) -> Self {
        Self { workspace }
    }
}

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Read the contents of a file. Provide the relative path from the workspace root."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path to the file"
                    },
                    "offset": {
                        "type": "integer",
                        "description": "Optional line offset to start reading from"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Optional maximum number of lines to read"
                    }
                },
                "required": ["path"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolOutput, ToolError> {
        let path = args["path"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'path' argument".to_string()))?;
        let resolved = self.workspace.resolve(path)?;

        // Check file size before reading to prevent OOM
        const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10MB
        let metadata = tokio::fs::metadata(&resolved).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get file metadata: {}", e)))?;
        if metadata.len() > MAX_FILE_SIZE {
            return Err(ToolError::ExecutionFailed(format!(
                "File too large ({} bytes). Maximum size is {} bytes.", metadata.len(), MAX_FILE_SIZE
            )));
        }

        let content = tokio::fs::read_to_string(&resolved).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read file: {}", e)))?;
        
        // Apply offset/limit if specified
        let mut lines: Vec<&str> = content.lines().collect();
        if let Some(offset) = args["offset"].as_u64() {
            lines = lines.into_iter().skip(offset as usize).collect();
        }
        if let Some(limit) = args["limit"].as_u64() {
            lines.truncate(limit as usize);
        }
        
        Ok(ToolOutput {
            content: lines.join("\n"),
            metadata: json!({"path": path}),
            terminate: false,
        })
    }
}
