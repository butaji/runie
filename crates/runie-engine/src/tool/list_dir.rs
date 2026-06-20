//! ListDir tool — lists directory contents.

use crate::tool::{Tool, ToolContext, ToolOutput, ToolStatus};
use anyhow::Result;
use async_trait::async_trait;
use runie_core::tool::{resolve_path, tool_error};
use serde_json::Value;
use std::time::Instant;
use tokio::fs;

pub struct ListDirTool;

#[async_trait]
impl Tool for ListDirTool {
    fn name(&self) -> &str {
        "list_dir"
    }

    fn description(&self) -> &str {
        "List files and directories at a given path."
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Directory path to list (default: current directory)"
                }
            }
        })
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn requires_approval(&self, _input: &Value) -> bool {
        false
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolOutput> {
        let start = Instant::now();
        let path = input["path"].as_str().unwrap_or(".");
        let full_path = resolve_path(path, &ctx.working_dir);
        list_dir_impl(&full_path, start).await
    }
}

async fn list_dir_impl(path: &std::path::Path, start: Instant) -> Result<ToolOutput> {
    let mut entries = match fs::read_dir(path).await {
        Ok(e) => e,
        Err(e) => {
            return Ok(tool_error(
                "list_dir",
                &format!("Error listing {}: {}", path.display(), e),
                start,
                false,
            ))
        }
    };
    let mut lines = Vec::new();
    while let Ok(Some(entry)) = entries.next_entry().await {
        lines.push(format_dir_entry(&entry).await);
    }
    let content = if lines.is_empty() {
        "(empty directory)".to_string()
    } else {
        lines.join("\n")
    };
    Ok(ToolOutput {
        tool_name: "list_dir".to_string(),
        tool_args: serde_json::json!({ "path": path }),
        content,
        bytes_transferred: None,
        duration: start.elapsed(),
        status: ToolStatus::Success,
    })
}

async fn format_dir_entry(entry: &tokio::fs::DirEntry) -> String {
    let name = entry.file_name().to_string_lossy().to_string();
    let typ = if entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false) {
        "dir"
    } else {
        "file"
    };
    format!("{} ({})", name, typ)
}
