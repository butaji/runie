//! WriteFile tool — writes content to a file.

use crate::tool::{ToolContext, ToolOutput};
use runie_core::path::resolve_path_in;
use runie_core::tool::ToolDef;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
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

impl ToolDef for WriteFileTool {
    type Input = WriteFileInput;

    const NAME: &'static str = "write_file";
    const DESCRIPTION: &'static str = "Write content to a file, creating parent directories as needed.";
    const READ_ONLY: bool = false;
    const REQUIRES_APPROVAL: bool = true;

    async fn execute(input: Self::Input, ctx: &ToolContext) -> ToolOutput {
        let full_path = resolve_path_in(&input.path, &ctx.working_dir);
        let tool_args = serde_json::json!({ "path": input.path, "content": "<redacted>" });

        if let Err(e) = ensure_parent_dirs(&full_path).await {
            return ToolOutput::error(
                "write_file",
                tool_args,
                format!("Error creating parent directories: {}", e),
            );
        }

        match fs::write(&full_path, &input.content).await {
            Ok(()) => ToolOutput::success_with_bytes(
                "write_file",
                tool_args,
                format!(
                    "Wrote {} bytes to {}",
                    input.content.len(),
                    full_path.display()
                ),
                input.content.len() as u64,
            ),
            Err(e) => ToolOutput::error(
                "write_file",
                tool_args,
                format!("Error writing {}: {}", full_path.display(), e),
            ),
        }
    }
}

async fn ensure_parent_dirs(full_path: &std::path::Path) -> std::io::Result<()> {
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
    use crate::tool::ToolStatus;

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
        let input = WriteFileInput {
            path: file.to_string_lossy().to_string(),
            content: "hello".to_string(),
        };

        let out = WriteFileTool::execute(input, &ctx()).await;

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
        let input = WriteFileInput {
            path: file.to_string_lossy().to_string(),
            content: "hello".to_string(),
        };

        let out = WriteFileTool::execute(input, &ctx()).await;

        assert_eq!(out.status, ToolStatus::Error);
        assert!(out.content.contains("Error creating parent directories"));
        assert!(!file.exists());
    }
}
