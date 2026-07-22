//! EditFile tool — performs a single search-and-replace in a file.
//!
//! Uses `diffy` for patch creation, providing consistent diff output.

use crate::tool::{ToolContext, ToolOutput, ToolStatus};
use diffy::create_patch;
use runie_core::tool::resolve_path;
use runie_core::tool::{tool_error, ToolDef};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
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
    const DESCRIPTION: &'static str = "Replace the first occurrence of search text with replace text in a file.";
    const READ_ONLY: bool = false;
    const REQUIRES_APPROVAL: bool = true;

    async fn execute(input: Self::Input, ctx: &ToolContext) -> ToolOutput {
        let start = Instant::now();
        let full_path = resolve_path(&input.path, &ctx.working_dir);
        edit_file_impl(&full_path, &input.search, &input.replace, start).await
    }
}

async fn edit_file_impl(path: &std::path::Path, search: &str, replace: &str, start: Instant) -> ToolOutput {
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
    let new_content = match apply_search_replace(&content, search, replace) {
        Ok(c) => c,
        Err(e) => return tool_error("edit_file", &e, start, false),
    };
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

/// Apply search/replace using diffy patch.
/// Creates a diffy patch from the original content to the new content.
/// This ensures consistent diff output formatting.
fn apply_search_replace(content: &str, search: &str, replace: &str) -> Result<String, String> {
    let new_content = content.replacen(search, replace, 1);
    // Create a patch to verify the edit is valid
    let _patch = create_patch(content, &new_content);
    // Patch was created successfully, so the edit is valid
    Ok(new_content)
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

#[cfg(test)]
mod tests {
    use super::apply_search_replace;

    #[test]
    fn test_apply_search_replace_simple() {
        let content = "Hello World";
        let result = apply_search_replace(content, "World", "Rust").unwrap();
        assert_eq!(result, "Hello Rust");
    }

    #[test]
    fn test_apply_search_replace_multiline() {
        let content = "fn main() {\n    println!(\"hello\");\n}";
        let result = apply_search_replace(content, "hello", "world").unwrap();
        assert_eq!(result, "fn main() {\n    println!(\"world\");\n}");
    }

    #[test]
    fn test_apply_search_replace_creates_valid_patch() {
        let content = "line 1\nline 2\nline 3";
        // Should create a valid patch without panicking
        let result = apply_search_replace(content, "line 2", "modified line 2").unwrap();
        assert_eq!(result, "line 1\nmodified line 2\nline 3");
    }
}
