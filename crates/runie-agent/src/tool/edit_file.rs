//! EditFile tool — performs a single search-and-replace in a file.

use crate::tool::{ToolContext, ToolOutput, ToolStatus};
use runie_core::path::resolve_path_in;
use runie_core::tool::{tool_error, ToolDef};
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
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

impl ToolDef for EditFileTool {
    type Input = EditFileInput;

    const NAME: &'static str = "edit_file";
    const DESCRIPTION: &'static str =
        "Replace the first occurrence of search text with replace text in a file.";
    const READ_ONLY: bool = false;
    const REQUIRES_APPROVAL: bool = true;

    async fn execute(input: Self::Input, ctx: &ToolContext) -> ToolOutput {
        let start = Instant::now();
        let full_path = resolve_path_in(&input.path, &ctx.working_dir);
        edit_file_impl(&full_path, &input.search, &input.replace, start).await
    }
}

async fn edit_file_impl(
    path: &std::path::Path,
    search: &str,
    replace: &str,
    start: Instant,
) -> ToolOutput {
    if search.is_empty() {
        return tool_error(
            "edit_file",
            "Error: search text cannot be empty",
            start,
            false,
        );
    }
    let content = match read_file(path).await {
        Ok(c) => c,
        Err(e) => {
            return tool_error(
                "edit_file",
                &format!("Error reading {}: {}", path.display(), e),
                start,
                false,
            )
        }
    };
    if let Some(err) = validate_match_count(&content, search, path) {
        return tool_error("edit_file", &err, start, false);
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
) -> ToolOutput {
    match fs::write(path, new_content).await {
        Ok(()) => build_edit_output(path, search, replace, new_content, start),
        Err(e) => tool_error(
            "edit_file",
            &format!("Error writing {}: {}", path.display(), e),
            start,
            false,
        ),
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
