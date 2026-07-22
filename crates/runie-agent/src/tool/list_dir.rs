//! ListDir tool — lists directory contents.

use crate::tool::{ToolContext, ToolOutput, ToolStatus};
use runie_core::tool::resolve_path;
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
        let full_path = resolve_path(path_str, &ctx.working_dir);
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
            format!("{} is empty (0 entries)", full_path.display())
        } else {
            let listing = names.join("\n");
            // Cap output so huge directories don't blow up the follow-up
            // request (some providers stall for minutes on very large
            // tool results). ~2000 lines / 50KB, matching pi's limits.
            let truncated = runie_core::tool::truncate_output(&listing, 50_000, 2_000);
            let shown = truncated.lines().filter(|l| *l != "…").count();
            if shown < names.len() {
                format!(
                    "Contents of {} ({} of {} entries shown):\n{}",
                    full_path.display(),
                    shown,
                    names.len(),
                    truncated
                )
            } else {
                format!("Contents of {} ({} entries):\n{}", full_path.display(), names.len(), listing)
            }
        };
        ToolOutput {
            tool_name: "list_dir".into(),
            tool_args,
            content,
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
        let input = ListDirInput { path: Some(".".to_string()) };
        let ctx = ToolContext::default();
        let result = ListDirTool::execute(input, &ctx).await;
        assert_eq!(result.status, ToolStatus::Success);
    }

    #[tokio::test]
    async fn output_includes_path_and_count_header() {
        let dir = std::env::temp_dir().join(format!("runie_list_dir_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.txt"), "x").unwrap();
        let input = ListDirInput { path: Some(dir.to_string_lossy().into_owned()) };
        let ctx = ToolContext::default();
        let result = ListDirTool::execute(input, &ctx).await;
        assert_eq!(result.status, ToolStatus::Success);
        assert!(
            result.content.contains(&dir.to_string_lossy().to_string()),
            "output should name the listed path: {}",
            result.content
        );
        assert!(result.content.contains("1 entries"), "got: {}", result.content);
        assert!(result.content.contains("a.txt"), "got: {}", result.content);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn output_is_capped_for_huge_directories() {
        let dir = std::env::temp_dir().join(format!("runie_list_dir_cap_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for i in 0..2_100 {
            std::fs::write(dir.join(format!("f{i:04}.txt")), "x").unwrap();
        }
        let input = ListDirInput { path: Some(dir.to_string_lossy().into_owned()) };
        let ctx = ToolContext::default();
        let result = ListDirTool::execute(input, &ctx).await;
        assert_eq!(result.status, ToolStatus::Success);
        assert!(
            result.content.contains("of 2100 entries shown"),
            "truncation note missing: {}",
            &result.content[..200.min(result.content.len())]
        );
        assert!(result.content.len() < 60_000, "output not capped");
        std::fs::remove_dir_all(&dir).ok();
    }
}
