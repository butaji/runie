//! ReadFile tool — reads file contents with optional offset/limit.

use crate::tool::{ToolContext, ToolOutput};
use runie_core::tool::resolve_path;
use runie_core::tool::ToolDef;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use tokio::fs;

/// Update frecency when a file is successfully read.
fn record_file_access(path: &std::path::Path) {
    if let Some(state) = runie_core::actors::FffSearchState::get() {
        state.record_file_access(path);
    }
}

/// Input parameters for read_file tool.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ReadFileInput {
    /// Path to the file to read
    pub path: String,
    /// Starting line number (0-based)
    #[serde(default)]
    pub offset: Option<u64>,
    /// Maximum number of lines to read
    #[serde(default)]
    pub limit: Option<u64>,
}

pub struct ReadFileTool;

impl ReadFileTool {
    /// Read file contents from disk.
    async fn read_file(path: &std::path::Path) -> Result<String, ToolOutput> {
        fs::read_to_string(path).await.map_err(|e| {
            ToolOutput::error(
                "read_file",
                serde_json::json!({ "path": path.to_string_lossy() }),
                format!("Error reading {}: {}", path.display(), e),
            )
        })
    }

    /// Slice content by offset and limit.
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
}

impl ToolDef for ReadFileTool {
    type Input = ReadFileInput;

    const NAME: &'static str = "read_file";
    const DESCRIPTION: &'static str =
        "Read the contents of a file from disk. Supports optional offset and limit.";
    const READ_ONLY: bool = true;
    const REQUIRES_APPROVAL: bool = false;

    async fn execute(input: Self::Input, ctx: &ToolContext) -> ToolOutput {
        if input.path.is_empty() {
            return ToolOutput::error(
                "read_file",
                serde_json::json!({ "path": "" }),
                "path is required".to_string(),
            );
        }

        let full_path = resolve_path(&input.path, &ctx.working_dir);
        let content = match Self::read_file(&full_path).await {
            Ok(c) => {
                record_file_access(&full_path);
                c
            }
            Err(e) => return e,
        };

        let offset = input.offset.map(|v| v as usize);
        let limit = input.limit.map(|v| v as usize);
        let output = Self::slice_content(&content, offset, limit);
        ToolOutput::success_with_bytes(
            "read_file",
            serde_json::json!({ "path": input.path }),
            output,
            content.len() as u64,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_deserializes_required() {
        let json = serde_json::json!({ "path": "/tmp/test.txt" });
        let input: ReadFileInput = serde_json::from_value(json).unwrap();
        assert_eq!(input.path, "/tmp/test.txt");
        assert_eq!(input.offset, None);
        assert_eq!(input.limit, None);
    }

    #[test]
    fn input_deserializes_full() {
        let json = serde_json::json!({
            "path": "/tmp/test.txt",
            "offset": 10,
            "limit": 50
        });
        let input: ReadFileInput = serde_json::from_value(json).unwrap();
        assert_eq!(input.path, "/tmp/test.txt");
        assert_eq!(input.offset, Some(10));
        assert_eq!(input.limit, Some(50));
    }

    #[test]
    fn input_schema_generates() {
        let schema = runie_core::tool::generate_schema::<ReadFileInput>();
        assert!(schema.is_object());
    }

    #[test]
    fn slice_content_full() {
        let content = "line1\nline2\nline3\nline4\nline5";
        let result = ReadFileTool::slice_content(content, None, None);
        assert_eq!(result, content);
    }

    #[test]
    fn slice_content_with_offset() {
        let content = "line1\nline2\nline3\nline4\nline5";
        let result = ReadFileTool::slice_content(content, Some(1), None);
        assert!(result.contains("line2"));
        assert!(result.contains("line5"));
    }

    #[test]
    fn slice_content_with_limit() {
        let content = "line1\nline2\nline3\nline4\nline5";
        let result = ReadFileTool::slice_content(content, None, Some(2));
        assert!(result.contains("line1"));
        assert!(result.contains("line2"));
    }
}
