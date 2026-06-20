//! EditFile tool — performs a single search-and-replace in a file.

use crate::tool::{Tool, ToolContext, ToolOutput, ToolStatus};
use anyhow::Result;
use async_trait::async_trait;
use runie_core::tool::{resolve_path, tool_error};
use serde_json::Value;
use std::time::Instant;
use tokio::fs;

pub struct EditFileTool;

#[async_trait]
impl Tool for EditFileTool {
    fn name(&self) -> &str {
        "edit_file"
    }

    fn description(&self) -> &str {
        "Replace the first occurrence of search text with replace text in a file."
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to edit"
                },
                "search": {
                    "type": "string",
                    "description": "Text to search for (must match exactly once)"
                },
                "replace": {
                    "type": "string",
                    "description": "Text to replace the search text with"
                }
            },
            "required": ["path", "search", "replace"]
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
        let (path, search, replace) = parse_input(&input)?;
        let full_path = resolve_path(&path, &ctx.working_dir);
        edit_file_impl(&full_path, &search, &replace, start).await
    }
}

fn parse_input(input: &Value) -> Result<(String, String, String)> {
    let path = input["path"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("path is required"))?
        .to_string();
    let search = input["search"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("search is required"))?
        .to_string();
    let replace = input["replace"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("replace is required"))?
        .to_string();
    Ok((path, search, replace))
}

async fn edit_file_impl(
    path: &std::path::Path,
    search: &str,
    replace: &str,
    start: Instant,
) -> Result<ToolOutput> {
    if search.is_empty() {
        return Ok(tool_error(
            "edit_file",
            "Error: search text cannot be empty",
            start,
            false,
        ));
    }
    let content = match read_file(path).await {
        Ok(c) => c,
        Err(e) => {
            return Ok(tool_error(
                "edit_file",
                &format!("Error reading {}: {}", path.display(), e),
                start,
                false,
            ))
        }
    };
    if let Some(err) = validate_match_count(&content, search, path) {
        return Ok(tool_error("edit_file", &err, start, false));
    }
    let new_content = content.replacen(search, replace, 1);
    write_edited_file(path, search, replace, &new_content, start).await
}

async fn read_file(path: &std::path::Path) -> Result<String, std::io::Error> {
    fs::read_to_string(path).await
}

async fn write_edited_file(
    path: &std::path::Path,
    search: &str,
    replace: &str,
    new_content: &str,
    start: Instant,
) -> Result<ToolOutput> {
    match fs::write(path, new_content).await {
        Ok(()) => Ok(build_edit_output(path, search, replace, new_content, start)),
        Err(e) => Ok(tool_error(
            "edit_file",
            &format!("Error writing {}: {}", path.display(), e),
            start,
            false,
        )),
    }
}

fn build_edit_output(
    path: &std::path::Path,
    search: &str,
    replace: &str,
    new_content: &str,
    start: Instant,
) -> ToolOutput {
    ToolOutput {
        tool_name: "edit_file".to_string(),
        tool_args: serde_json::json!({ "path": path.to_string_lossy(), "search": search, "replace": replace }),
        content: format!(
            "Replaced 1 occurrence in {}\n\nBefore:\n{}\n\nAfter:\n{}",
            path.display(),
            search,
            replace
        ),
        bytes_transferred: Some(new_content.len() as u64),
        duration: start.elapsed(),
        status: ToolStatus::Success,
    }
}

fn validate_match_count(content: &str, search: &str, path: &std::path::Path) -> Option<String> {
    let count = content.matches(search).count();
    if count == 0 {
        Some(format!(
            "Error: search text not found in {}",
            path.display()
        ))
    } else if count > 1 {
        Some(format!(
            "Error: search text appears {} times in {}. Be more specific.",
            count,
            path.display()
        ))
    } else {
        None
    }
}
