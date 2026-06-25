//! ListDir tool — lists directory contents.

use crate::define_tool;
use crate::tool::{Tool, ToolContext, ToolOutput, ToolStatus};
use anyhow::Result;
use async_trait::async_trait;
use runie_core::path::resolve_path_in;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::time::Instant;

/// Input parameters for list_dir tool (schema-derived).
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

    /// Generate JSON schema for this tool's input.
    pub fn input_schema() -> Value {
        runie_core::tool::generate_schema::<ListDirInput>()
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

        // Parse typed input, falling back to raw access for backward compat
        let typed: Result<ListDirInput, _> = serde_json::from_value(input.clone());
        let path_str = match typed {
            Ok(inp) => inp.path.unwrap_or_else(|| ".".to_string()),
            Err(_) => input["path"]
                .as_str()
                .map(String::from)
                .unwrap_or_else(|| ".".to_string()),
        };

        let full_path = resolve_path_in(&path_str, &ctx.working_dir);
        let tool_args = serde_json::json!({ "path": path_str });

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
        let schema = ListDirTool::input_schema();
        assert!(schema.is_object());
    }

    #[tokio::test]
    async fn tool_call_executes() {
        let input = serde_json::json!({ "path": "." });
        let ctx = ToolContext::default();
        let tool = ListDirTool;
        let result = tool.call(input, &ctx).await;
        assert!(result.is_ok());
    }
}
