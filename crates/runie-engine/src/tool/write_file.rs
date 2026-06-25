//! WriteFile tool — writes content to a file.

use crate::define_tool;
use crate::tool::{Tool, ToolContext, ToolOutput};
use anyhow::Result;
use async_trait::async_trait;
use runie_core::path::resolve_path_in;
use serde_json::Value;
use tokio::fs;

pub struct WriteFileTool;

#[allow(clippy::use_self)]
#[async_trait]
impl Tool for WriteFileTool {
    define_tool! {
        name: "write_file",
        description: "Write content to a file, creating parent directories as needed.",
        read_only: false,
        approval: true,
        fields: {
            "path": ("string", "Path to the file to write"),
            "content": ("string", "Content to write to the file")
        },
        required: ["path", "content"]
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolOutput> {
        let path = input["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("path is required"))?;
        let content = input["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("content is required"))?;
        let full_path = resolve_path_in(path, &ctx.working_dir);
        let tool_args = serde_json::json!({ "path": path, "content": "<redacted>" });

        if let Err(e) = ensure_parent_dirs(&full_path).await {
            return Ok(ToolOutput::error(
                "write_file",
                tool_args,
                format!("Error creating parent directories: {}", e),
            ));
        }

        match fs::write(&full_path, content).await {
            Ok(()) => Ok(ToolOutput::success_with_bytes(
                "write_file",
                tool_args,
                format!("Wrote {} bytes to {}", content.len(), full_path.display()),
                content.len() as u64,
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
