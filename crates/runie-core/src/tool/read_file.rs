//! ReadFile tool — reads file contents with optional offset/limit.

use crate::tool::{Tool, ToolContext, ToolOutput, ToolStatus};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::time::Instant;

/// Update frecency when a file is successfully read.
fn record_file_access(path: &std::path::Path) {
    if let Some(state) = crate::actors::FffSearchState::get() {
        state.record_file_access(path);
    }
}

pub struct ReadFileTool;

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Read the contents of a file from disk. Supports optional offset and limit."
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to read"
                },
                "offset": {
                    "type": "integer",
                    "description": "Starting line number (0-based)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of lines to read"
                }
            },
            "required": ["path"]
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
        let path = input["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("path is required"))?;
        let offset = input["offset"].as_u64().map(|v| v as usize);
        let limit = input["limit"].as_u64().map(|v| v as usize);

        let full_path = resolve_path(path, &ctx.working_dir);
        let content = match std::fs::read_to_string(&full_path) {
            Ok(c) => {
                record_file_access(&full_path);
                c
            }
            Err(e) => {
                return Ok(ToolOutput {
                    tool_name: "read_file".to_string(),
                    tool_args: serde_json::json!({ "path": path }),
                    content: format!("Error reading {}: {}", full_path.display(), e),
                    bytes_transferred: None,
                    duration: start.elapsed(),
                    status: ToolStatus::Error,
                });
            }
        };

        let output = slice_content(&content, offset, limit);
        Ok(ToolOutput {
            tool_name: "read_file".to_string(),
            tool_args: serde_json::json!({ "path": path, "offset": offset, "limit": limit }),
            content: output,
            bytes_transferred: Some(content.len() as u64),
            duration: start.elapsed(),
            status: ToolStatus::Success,
        })
    }
}

/// Resolve a path relative to working_dir if not absolute.
fn resolve_path(path: &str, working_dir: &std::path::Path) -> std::path::PathBuf {
    let p = std::path::Path::new(path);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        working_dir.join(p)
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
        return "(end of file)".to_string();
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
