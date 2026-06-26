//! ListDir tool — lists directory contents.

use crate::tool::{Tool, ToolContext, ToolOutput, ToolStatus};
use anyhow::Result;
use async_trait::async_trait;
use runie_core::path::resolve_path_in;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
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

    /// Generate JSON schema for this tool's input.
    pub fn input_schema() -> Value {
        runie_core::tool::generate_schema::<ListDirInput>()
    }
}

#[async_trait]
impl Tool for ListDirTool {
    fn name(&self) -> &str { "list_dir" }
    fn description(&self) -> &str { "List files and directories in a given path." }
    fn input_schema(&self) -> Value {
        Self::input_schema()
    }
    fn is_read_only(&self) -> bool { true }
    fn requires_approval(&self, _input: &Value) -> bool { false }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolOutput> {
        let start = Instant::now();
        let typed: ListDirInput = serde_json::from_value(input)?;
        let path_str = typed.path.as_deref().unwrap_or(".");
        let full_path = resolve_path_in(path_str, &ctx.working_dir);
        let tool_args = serde_json::json!({ "path": path_str });

        let dir = tokio::fs::read_dir(&full_path).await?;
        let names = Self::collect_entries(dir).await;
        let content = if names.is_empty() {
            "(empty directory)"
        } else {
            &names.join("\n")
        };
        Ok(ToolOutput {
            tool_name: "list_dir".into(),
            tool_args,
            content: content.to_string(),
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
