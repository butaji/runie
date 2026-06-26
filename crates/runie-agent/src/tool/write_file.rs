//! WriteFile tool — writes content to a file.

use crate::tool::{Tool, ToolContext, ToolOutput};
use anyhow::Result;
use async_trait::async_trait;
use runie_core::path::resolve_path_in;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use tokio::fs;

/// Input parameters for write_file tool.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct WriteFileInput {
    /// Path to the file to write
    pub path: String,
    /// Content to write to the file
    pub content: String,
}

pub struct WriteFileTool;

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &str { "write_file" }
    fn description(&self) -> &str {
        "Write content to a file, creating parent directories as needed."
    }
    fn input_schema(&self) -> Value {
        runie_core::tool::generate_schema::<WriteFileInput>()
    }
    fn is_read_only(&self) -> bool { false }
    fn requires_approval(&self, _input: &Value) -> bool { true }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolOutput> {
        let typed: WriteFileInput = serde_json::from_value(input)?;
        let full_path = resolve_path_in(&typed.path, &ctx.working_dir);
        let tool_args = serde_json::json!({ "path": typed.path, "content": "<redacted>" });

        if let Err(e) = ensure_parent_dirs(&full_path).await {
            return Ok(ToolOutput::error(
                "write_file",
                tool_args,
                format!("Error creating parent directories: {}", e),
            ));
        }

        match fs::write(&full_path, &typed.content).await {
            Ok(()) => Ok(ToolOutput::success_with_bytes(
                "write_file",
                tool_args,
                format!(
                    "Wrote {} bytes to {}",
                    typed.content.len(),
                    full_path.display()
                ),
                typed.content.len() as u64,
            )),
            Err(e) => Ok(ToolOutput::error(
                "write_file",
                tool_args,
                format!("Error writing {}: {}", full_path.display(), e),
            )),
        }
    }
}

async fn ensure_parent_dirs(full_path: &std::path::Path) -> Result<()> {
    if let Some(parent) = full_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::ToolStatus;

    fn ctx() -> ToolContext {
        ToolContext {
            working_dir: std::env::current_dir().unwrap(),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn write_file_creates_file_and_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("nested/dir/file.txt");
        let tool = WriteFileTool;
        let input = serde_json::json!({
            "path": file.to_string_lossy(),
            "content": "hello"
        });

        let out = tool.call(input, &ctx()).await.unwrap();

        assert_eq!(out.status, ToolStatus::Success);
        assert!(file.exists());
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "hello");
    }

    #[tokio::test]
    async fn write_file_reports_parent_dir_creation_error() {
        let dir = tempfile::tempdir().unwrap();
        // Create a file where we expect a parent directory, forcing create_dir_all to fail.
        let blocking = dir.path().join("blocking");
        std::fs::write(&blocking, "x").unwrap();
        let file = blocking.join("file.txt");
        let tool = WriteFileTool;
        let input = serde_json::json!({
            "path": file.to_string_lossy(),
            "content": "hello"
        });

        let out = tool.call(input, &ctx()).await.unwrap();

        assert_eq!(out.status, ToolStatus::Error);
        assert!(out.content.contains("Error creating parent directories"));
        assert!(!file.exists());
    }
}
