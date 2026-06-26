//! EditFile tool — performs a single search-and-replace in a file.

use crate::tool::{tool_error, Tool, ToolContext, ToolOutput, ToolStatus};
use anyhow::Result;
use async_trait::async_trait;
use runie_core::path::resolve_path_in;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::time::Instant;
use tokio::fs;

/// Input parameters for edit_file tool.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct EditFileInput {
    /// Path to the file to edit
    pub path: String,
    /// Text to search for (must match exactly once)
    pub search: String,
    /// Text to replace the search text with
    pub replace: String,
}

pub struct EditFileTool;

#[async_trait]
impl Tool for EditFileTool {
    fn name(&self) -> &str { "edit_file" }
    fn description(&self) -> &str {
        "Replace the first occurrence of search text with replace text in a file."
    }
    fn input_schema(&self) -> Value {
        runie_core::tool::generate_schema::<EditFileInput>()
    }
    fn is_read_only(&self) -> bool { false }
    fn requires_approval(&self, _input: &Value) -> bool { true }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolOutput> {
        let start = Instant::now();
        let typed: EditFileInput = serde_json::from_value(input)?;
        let full_path = resolve_path_in(&typed.path, &ctx.working_dir);
        edit_file_impl(&full_path, &typed.search, &typed.replace, start).await
    }
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
        tool_name: "edit_file".to_owned(),
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
