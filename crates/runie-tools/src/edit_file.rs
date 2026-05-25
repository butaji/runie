use async_trait::async_trait;
use runie_core::{Tool, ToolSchema, ToolOutput, ToolError};
use serde_json::json;
use std::time::UNIX_EPOCH;
use tokio::fs;
use crate::Workspace;

pub struct EditFileTool {
    workspace: Workspace,
}

impl EditFileTool {
    pub fn new(workspace: Workspace) -> Self {
        Self { workspace }
    }

    /// Get the modification time of a file (async).
    async fn get_mtime(path: &std::path::Path) -> Result<u64, ToolError> {
        let metadata = fs::metadata(path)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get file metadata: {}", e)))?;
        let modified = metadata.modified()
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get modification time: {}", e)))?;
        let duration = modified.duration_since(UNIX_EPOCH)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to compute duration: {}", e)))?;
        Ok(duration.as_secs())
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
                    },
                    "force": {
                        "type": "boolean",
                        "description": "If true, replace all occurrences. If false (default), fail when multiple matches exist."
                    }
                },
                "required": ["path", "old_string", "new_string"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolOutput, ToolError> {
        let old_string = args["old_string"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'old_string' argument".to_string()))?;
        let new_string = args["new_string"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'new_string' argument".to_string()))?;
        let path = args["path"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'path' argument".to_string()))?;
        let force = args["force"].as_bool().unwrap_or(false);

        let resolved = self.workspace.resolve(path)?;
        
        // Get mtime before reading (for stale detection)
        let read_mtime = Self::get_mtime(&resolved).await?;
        
        let content = tokio::fs::read_to_string(&resolved).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read file: {}", e)))?;

        let occurrences = content.matches(old_string).count();
        if occurrences == 0 {
            return Err(ToolError::ExecutionFailed(
                format!("String not found in file: {}", old_string)
            ));
        }
        if occurrences > 1 && !force {
            return Err(ToolError::ExecutionFailed(
                format!("String appears {} times. Use a more specific replacement.", occurrences)
            ));
        }

        let new_content = if force {
            content.replace(old_string, new_string)
        } else {
            content.replacen(old_string, new_string, 1)
        };
        let replacement_count = if force { occurrences } else { 1 };
        
        // Check if file was modified since we read it
        let current_mtime = Self::get_mtime(&resolved).await?;
        if current_mtime != read_mtime {
            return Err(ToolError::ExecutionFailed(
                format!("File '{}' was modified since it was read. Please re-read the file and try again.", path)
            ));
        }
        
        tokio::fs::write(&resolved, new_content).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to write file: {}", e)))?;

        Ok(ToolOutput {
            content: format!("Edited {} ({} replacement{})", path, replacement_count, if replacement_count == 1 { "" } else { "s" }),
            metadata: json!({
                "path": path,
                "old_content": old_string,
                "new_content": new_string,
                "replacement_count": replacement_count
            }),
            terminate: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    #[test]
    fn test_get_mtime_returns_u64() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let _ws = Workspace::new(std::path::PathBuf::from("."));
            // Just verify the method exists and works
            let mtime = EditFileTool::get_mtime(std::path::Path::new("Cargo.toml")).await;
            assert!(mtime.is_ok());
            assert!(mtime.unwrap() > 0);
        });
    }
}
