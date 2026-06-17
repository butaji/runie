//! WriteFile tool — writes content to a file.

use crate::tool::{Tool, ToolContext, ToolOutput, ToolStatus};
use anyhow::Result;
use async_trait::async_trait;
use runie_core::tool::resolve_path;
use serde_json::Value;
use std::time::Instant;

pub struct WriteFileTool;

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }

    fn description(&self) -> &str {
        "Write content to a file, creating parent directories as needed."
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to write"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file"
                }
            },
            "required": ["path", "content"]
        })
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn requires_approval(&self, _input: &Value) -> bool {
        true
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolOutput> {
        let start = Instant::now();
        let path = input["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("path is required"))?;
        let content = input["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("content is required"))?;
        let full_path = resolve_path(path, &ctx.working_dir);

        if let Err(e) = ensure_parent_dirs(&full_path) {
            return Ok(output_error(
                "write_file",
                path,
                &format!("Error creating parent directories: {}", e),
                start,
            ));
        }

        write_and_return(path, &full_path, content, start)
    }
}

fn ensure_parent_dirs(full_path: &std::path::Path) -> Result<()> {
    if let Some(parent) = full_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    Ok(())
}

fn write_and_return(
    path: &str,
    full_path: &std::path::Path,
    content: &str,
    start: Instant,
) -> Result<ToolOutput> {
    match std::fs::write(full_path, content) {
        Ok(()) => Ok(ToolOutput {
            tool_name: "write_file".to_string(),
            tool_args: serde_json::json!({ "path": path, "content": "<redacted>" }),
            content: format!("Wrote {} bytes to {}", content.len(), full_path.display()),
            bytes_transferred: Some(content.len() as u64),
            duration: start.elapsed(),
            status: ToolStatus::Success,
        }),
        Err(e) => Ok(output_error(
            "write_file",
            path,
            &format!("Error writing {}: {}", full_path.display(), e),
            start,
        )),
    }
}

fn output_error(tool: &str, _path: &str, msg: &str, start: Instant) -> ToolOutput {
    ToolOutput {
        tool_name: tool.to_string(),
        tool_args: serde_json::Value::Null,
        content: msg.to_string(),
        bytes_transferred: None,
        duration: start.elapsed(),
        status: ToolStatus::Error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
