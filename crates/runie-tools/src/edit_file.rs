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

    fn extract_edit_args(args: serde_json::Value) -> Result<(String, String, String, bool), ToolError> {
        let old_string = args["old_string"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'old_string'".into()))?
            .to_string();
        let new_string = args["new_string"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'new_string'".into()))?
            .to_string();
        let path = args["path"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'path'".into()))?
            .to_string();
        let force = args["force"].as_bool().unwrap_or(false);
        Ok((old_string, new_string, path, force))
    }

    async fn read_and_validate(&self, resolved: &std::path::PathBuf, old_string: &str, force: bool) -> Result<(String, usize), ToolError> {
        let content = tokio::fs::read_to_string(resolved).await
            .map_err(|e| ToolError::ExecutionFailed(format!("read failed: {}", e)))?;
        let occurrences = content.matches(old_string).count();
        if occurrences == 0 {
            return Err(ToolError::ExecutionFailed(format!("String not found: {}", old_string)));
        }
        if occurrences > 1 && !force {
            return Err(ToolError::ExecutionFailed(
                format!("String appears {} times. Use a more specific replacement.", occurrences)
            ));
        }
        Ok((content, occurrences))
    }

    fn compute_replacement(old_string: &str, new_string: &str, force: bool, content: &str) -> (String, usize) {
        let new_content = if force {
            content.replace(old_string, new_string)
        } else {
            content.replacen(old_string, new_string, 1)
        };
        let replacement_count = if force { content.matches(old_string).count() } else { 1 };
        (new_content, replacement_count)
    }

    async fn check_stale_and_write(&self, resolved: &std::path::PathBuf, path: &str, new_content: &str) -> Result<(), ToolError> {
        let read_mtime = Self::get_mtime(resolved).await?;
        let current_mtime = Self::get_mtime(resolved).await?;
        if current_mtime != read_mtime {
            return Err(ToolError::ExecutionFailed(
                format!("File '{}' was modified since read. Please re-read and try again.", path)
            ));
        }
        tokio::fs::write(resolved, new_content).await
            .map_err(|e| ToolError::ExecutionFailed(format!("write failed: {}", e)))
    }
}

#[async_trait]
impl Tool for EditFileTool {
    fn name(&self) -> &str { "edit_file" }
    fn description(&self) -> &str { "Edit a file by replacing old_string with new_string." }
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative path to the file" },
                    "old_string": { "type": "string", "description": "Exact string to replace" },
                    "new_string": { "type": "string", "description": "Replacement string" },
                    "force": { "type": "boolean", "description": "If true, replace all occurrences." }
                },
                "required": ["path", "old_string", "new_string"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolOutput, ToolError> {
        let (old_string, new_string, path, force) = EditFileTool::extract_edit_args(args)?;
        let resolved = self.workspace.resolve(&path)?;
        let (content, _count) = self.read_and_validate(&resolved, &old_string, force).await?;
        let (new_content, replacement_count) = EditFileTool::compute_replacement(&old_string, &new_string, force, &content);
        self.check_stale_and_write(&resolved, &path, &new_content).await?;

        Ok(ToolOutput {
            content: format!("Edited {} ({} replacement{})", path, replacement_count, if replacement_count == 1 { "" } else { "s" }),
            metadata: json!({ "path": path, "old_content": old_string, "new_content": new_string, "replacement_count": replacement_count }),
            terminate: false,
        })
    }
}
