use async_trait::async_trait;
use runie_core::{Tool, ToolSchema, ToolOutput, ToolError};
use serde_json::json;
use crate::Workspace;

pub struct EditFileTool {
    workspace: Workspace,
}

impl EditFileTool {
    pub fn new(workspace: Workspace) -> Self {
        Self { workspace }
    }
}

#[async_trait]
impl Tool for EditFileTool {
    fn name(&self) -> &str {
        "edit_file"
    }

    fn description(&self) -> &str {
        "Edit a file by replacing old_string with new_string. Both must match exactly."
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
                    "old_string": {
                        "type": "string",
                        "description": "Exact string to replace"
                    },
                    "new_string": {
                        "type": "string",
                        "description": "Replacement string"
                    }
                },
                "required": ["path", "old_string", "new_string"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolOutput, ToolError> {
        let path = args["path"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'path' argument".to_string()))?;
        let old_string = args["old_string"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'old_string' argument".to_string()))?;
        let new_string = args["new_string"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'new_string' argument".to_string()))?;
        
        let resolved = self.workspace.resolve(path)?;
        let content = tokio::fs::read_to_string(&resolved).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read file: {}", e)))?;
        
        if !content.contains(old_string) {
            return Err(ToolError::InvalidArguments(
                format!("old_string not found in file: {}", path)
            ));
        }
        
        let new_content = content.replacen(old_string, new_string, 1);
        tokio::fs::write(&resolved, new_content).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to write file: {}", e)))?;
        
        Ok(ToolOutput {
            content: format!("File edited: {}", path),
            metadata: json!({
                "path": path,
                "old_content": old_string,
                "new_content": new_string
            }),
            terminate: false,
        })
    }
}
