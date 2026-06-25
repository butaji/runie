//! ReadFile tool — reads file contents with optional offset/limit.

use crate::define_tool;
use crate::tool::{Tool, ToolContext, ToolOutput};
use anyhow::Result;
use async_trait::async_trait;
use runie_core::path::resolve_path_in;
use serde_json::Value;
use tokio::fs;

/// Update frecency when a file is successfully read.
fn record_file_access(path: &std::path::Path) {
    if let Some(state) = runie_core::actors::FffSearchState::get() {
        state.record_file_access(path);
    }
}

pub struct ReadFileTool;

#[allow(clippy::use_self)]
#[async_trait]
impl Tool for ReadFileTool {
    define_tool! {
        name: "read_file",
        description: "Read the contents of a file from disk. Supports optional offset and limit.",
        read_only: true,
        approval: false,
        fields: {
            "path": ("string", "Path to the file to read"),
            "offset": ("integer", "Starting line number (0-based)"),
            "limit": ("integer", "Maximum number of lines to read")
        },
        required: ["path"]
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolOutput> {
        let path = input["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("path is required"))?;
        let offset = input["offset"].as_u64().map(|v| v as usize);
        let limit = input["limit"].as_u64().map(|v| v as usize);
        let tool_args = serde_json::json!({ "path": path });

        let full_path = resolve_path_in(path, &ctx.working_dir);
        let content = match fs::read_to_string(&full_path).await {
            Ok(c) => {
                record_file_access(&full_path);
                c
            }
            Err(e) => {
                return Ok(ToolOutput::error(
                    "read_file",
                    tool_args,
                    format!("Error reading {}: {}", full_path.display(), e),
                ));
            }
        };

        let output = slice_content(&content, offset, limit);
        let tool_args = serde_json::json!({ "path": path, "offset": offset, "limit": limit });
        Ok(ToolOutput::success_with_bytes(
            "read_file",
            tool_args,
            output,
            content.len() as u64,
        ))
    }
}

fn slice_content(content: &str, offset: Option<usize>, limit: Option<usize>) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();
    let start = offset.unwrap_or(0).min(total_lines);
    let end = limit
        .map(|l| (start + l).min(total_lines))
        .unwrap_or(total_lines);

    if start >= total_lines {
        return "(end of file)".to_owned();
    }

    let selected: String = lines[start..end].join("\n");
    let header = if offset.is_some() || limit.is_some() {
        format!("[Lines {}-{} of {}]\n", start + 1, end, total_lines)
    } else {
        String::new()
    };
    if end < total_lines {
        format!("{}{}\n[{} more lines]", header, selected, total_lines - end)
    } else {
        format!("{}{}", header, selected)
    }
}
