//! ListDir tool — lists directory contents.

use crate::define_tool;
use crate::tool::{Tool, ToolContext, ToolOutput, ToolStatus};
use anyhow::Result;
use async_trait::async_trait;
use runie_core::path::resolve_path_in;
use serde_json::Value;
use std::time::Instant;

pub struct ListDirTool;

impl ListDirTool {
    async fn collect_entries(dir: tokio::fs::ReadDir) -> Vec<String> {
        let mut names = Vec::new();
        let mut entries = dir;
        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name().to_string_lossy().to_string();
            let suffix = entry
                .file_type()
                .await
                .map(|ft| ft.is_dir())
                .unwrap_or(false);
            names.push(format!("{}{}", name, if suffix { "/" } else { "" }));
        }
        names.sort();
        names
    }
}

#[async_trait]
impl Tool for ListDirTool {
    define_tool! {
        name: "list_dir",
        description: "List files and directories in a given path.",
        read_only: true,
        approval: false,
        fields: {
            "path": ("string", "Directory path to list (default: current directory)")
        },
        required: []
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolOutput> {
        let start = Instant::now();
        let path = input["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("path is required"))?;
        let full_path = resolve_path_in(path, &ctx.working_dir);
        let tool_args = serde_json::json!({ "path": path });
        let dir = match tokio::fs::read_dir(&full_path).await {
            Ok(d) => d,
            Err(e) => {
                return Ok(ToolOutput {
                    tool_name: "list_dir".into(),
                    tool_args,
                    content: format!("Error reading directory {}: {}", full_path.display(), e),
                    bytes_transferred: None,
                    duration: start.elapsed(),
                    status: ToolStatus::Error,
                })
            }
        };
        let names = Self::collect_entries(dir).await;
        let content = if names.is_empty() {
            "(empty directory)".into()
        } else {
            names.join("\n")
        };
        Ok(ToolOutput {
            tool_name: "list_dir".into(),
            tool_args,
            content,
            bytes_transferred: None,
            duration: start.elapsed(),
            status: ToolStatus::Success,
        })
    }
}
