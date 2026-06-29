//! ListDir tool — lists directory contents.

use crate::tool::{ToolContext, ToolOutput, ToolStatus};
use runie_core::path::resolve_path_in;
use runie_core::tool::ToolDef;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::time::Instant;

/// Input parameters for list_dir tool.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ListDirInput {
    /// Directory path to list (default: current directory)
    #[serde(default)]
    pub path: Option<String>,
}

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

impl ToolDef for ListDirTool {
    type Input = ListDirInput;

    const NAME: &'static str = "list_dir";
    const DESCRIPTION: &'static str = "List files and directories in a given path.";
    const READ_ONLY: bool = true;
    const REQUIRES_APPROVAL: bool = false;

    async fn execute(input: Self::Input, ctx: &ToolContext) -> ToolOutput {
        let start = Instant::now();
        let path_str = input.path.as_deref().unwrap_or(".");
        let full_path = resolve_path_in(path_str, &ctx.working_dir);
        let tool_args = serde_json::json!({ "path": path_str });

        let dir = match tokio::fs::read_dir(&full_path).await {
            Ok(d) => d,
            Err(e) => {
                return ToolOutput::error(
                    "list_dir",
                    tool_args,
                    format!("Error reading directory {}: {}", full_path.display(), e),
                );
            }
        };
        let names = Self::collect_entries(dir).await;
        let content = if names.is_empty() {
            "(empty directory)"
        } else {
            &names.join("\n")
        };
        ToolOutput {
            tool_name: "list_dir".into(),
            tool_args,
            content: content.to_string(),
            bytes_transferred: None,
            duration: start.elapsed(),
            status: ToolStatus::Success,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_deserializes_minimal() {
        let json = serde_json::json!({});
        let input: ListDirInput = serde_json::from_value(json).unwrap();
        assert_eq!(input.path, None);
    }

    #[test]
    fn input_deserializes_with_path() {
        let json = serde_json::json!({ "path": "/tmp" });
        let input: ListDirInput = serde_json::from_value(json).unwrap();
        assert_eq!(input.path, Some("/tmp".into()));
    }

    #[test]
    fn input_schema_generates() {
        let schema = runie_core::tool::generate_schema::<ListDirInput>();
        assert!(schema.is_object());
    }

    #[tokio::test]
    async fn tool_call_executes() {
        let input = ListDirInput {
            path: Some(".".to_string()),
        };
        let ctx = ToolContext::default();
        let result = ListDirTool::execute(input, &ctx).await;
        assert_eq!(result.status, ToolStatus::Success);
    }
}
