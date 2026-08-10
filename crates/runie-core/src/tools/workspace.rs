//! Bounded, read-only workspace tools.

use std::path::Path;

use crate::types::{AgentTool, AgentToolResult, ToolResultContent};

pub const READ_MAX_LINES: usize = 1_000;
pub const READ_MAX_BYTES: usize = 100 * 1024;

pub struct ReadFileTool;

#[async_trait::async_trait]
impl AgentTool for ReadFileTool {
    fn name(&self) -> &str {
        "read"
    }
    fn label(&self) -> &str {
        "Read file"
    }
    fn description(&self) -> &str {
        "Read a text file from the working directory with bounded output."
    }
    fn parameters(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "line_offset": { "type": "integer", "minimum": 1 },
                "n_lines": { "type": "integer", "minimum": 1 }
            },
            "required": ["path"]
        }))
    }
    async fn execute(
        &self,
        _tool_call_id: &str,
        args: serde_json::Value,
        _signal: Option<tokio_util::sync::CancellationToken>,
        _on_update: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        let path = args
            .get("path")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "read requires a string `path` argument".to_owned())?;
        let content = tokio::fs::read_to_string(Path::new(path))
            .await
            .map_err(|error| format!("read {path:?}: {error}"))?;
        let start = args
            .get("line_offset")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(1)
            .saturating_sub(1) as usize;
        let requested = args
            .get("n_lines")
            .and_then(serde_json::Value::as_u64)
            .map(|n| n as usize)
            .unwrap_or(READ_MAX_LINES)
            .min(READ_MAX_LINES);
        let output = bounded_lines(&content, start, requested);
        Ok(AgentToolResult {
            content: vec![ToolResultContent::Text { text: output }],
            ..AgentToolResult::default()
        })
    }
}

fn bounded_lines(content: &str, start: usize, requested: usize) -> String {
    let mut output = String::new();
    for line in content.lines().skip(start).take(requested) {
        let addition = if output.is_empty() {
            line.to_owned()
        } else {
            format!("\n{line}")
        };
        if output.len() + addition.len() > READ_MAX_BYTES {
            break;
        }
        output.push_str(&addition);
    }
    if content.lines().skip(start).count() > requested || output.len() >= READ_MAX_BYTES {
        output.push_str("\n[output truncated]");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn read_returns_requested_line_range() {
        let path = std::env::temp_dir().join(format!("runie-read-{}.txt", std::process::id()));
        tokio::fs::write(&path, "one\ntwo\nthree\n").await.unwrap();
        let result = ReadFileTool
            .execute(
                "read-1",
                serde_json::json!({"path": path, "line_offset": 2, "n_lines": 1}),
                None,
                None,
            )
            .await
            .unwrap();
        let ToolResultContent::Text { text } = &result.content[0] else {
            panic!("expected text")
        };
        assert_eq!(text, "two\n[output truncated]");
        tokio::fs::remove_file(path).await.unwrap();
    }

    #[tokio::test]
    async fn read_rejects_missing_path() {
        let error = ReadFileTool
            .execute("read-1", serde_json::json!({}), None, None)
            .await
            .unwrap_err();
        assert!(error.contains("requires"));
    }
}
