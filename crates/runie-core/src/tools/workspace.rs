//! Bounded, read-only workspace tools.

use std::path::Path;

use crate::types::{AgentTool, AgentToolResult, ToolResultContent};

pub const READ_MAX_LINES: usize = 1_000;
pub const READ_MAX_BYTES: usize = 100 * 1024;

pub struct ReadFileTool;
pub struct WriteFileTool;
pub struct EditFileTool;
pub struct GrepTool;
pub struct GlobTool;

macro_rules! workspace_tool {
    ($name:literal, $label:literal, $description:literal, $schema:expr) => {
        fn name(&self) -> &str {
            $name
        }
        fn label(&self) -> &str {
            $label
        }
        fn description(&self) -> &str {
            $description
        }
        fn parameters(&self) -> Option<serde_json::Value> {
            Some($schema)
        }
    };
}

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

#[async_trait::async_trait]
impl AgentTool for WriteFileTool {
    workspace_tool!(
        "write",
        "Write file",
        "Create or replace a text file.",
        serde_json::json!({"type":"object","properties":{"path":{"type":"string"},"content":{"type":"string"}},"required":["path","content"]})
    );
    async fn execute(
        &self,
        _id: &str,
        args: serde_json::Value,
        _signal: Option<tokio_util::sync::CancellationToken>,
        _update: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        let path = required_string(&args, "path", "write")?;
        let content = required_string(&args, "content", "write")?;
        tokio::fs::write(path, content)
            .await
            .map_err(|error| error.to_string())?;
        text_result("written")
    }
}

#[async_trait::async_trait]
impl AgentTool for EditFileTool {
    workspace_tool!(
        "edit",
        "Edit file",
        "Replace one exact text occurrence in a file.",
        serde_json::json!({"type":"object","properties":{"path":{"type":"string"},"old_string":{"type":"string"},"new_string":{"type":"string"},"replace_all":{"type":"boolean"}},"required":["path","old_string","new_string"]})
    );
    async fn execute(
        &self,
        _id: &str,
        args: serde_json::Value,
        _signal: Option<tokio_util::sync::CancellationToken>,
        _update: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        let path = required_string(&args, "path", "edit")?;
        let old = required_string(&args, "old_string", "edit")?;
        let new = required_string(&args, "new_string", "edit")?;
        apply_edit(path, old, new, replace_all(&args)).await?;
        text_result("edited")
    }
}

fn replace_all(args: &serde_json::Value) -> bool {
    args.get("replace_all")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

async fn apply_edit(path: &str, old: &str, new: &str, all: bool) -> Result<(), String> {
    if old == new {
        return Err("old_string and new_string must differ".into());
    }
    let source = tokio::fs::read_to_string(path)
        .await
        .map_err(|error| error.to_string())?;
    let count = source.matches(old).count();
    if count == 0 {
        return Err("old_string was not found".into());
    }
    if count > 1 && !all {
        return Err(format!("old_string matched {count} times; set replace_all"));
    }
    let output = if all {
        source.replace(old, new)
    } else {
        source.replacen(old, new, 1)
    };
    tokio::fs::write(path, output)
        .await
        .map_err(|error| error.to_string())
}

#[async_trait::async_trait]
impl AgentTool for GrepTool {
    workspace_tool!(
        "grep",
        "Search files",
        "Search text files recursively.",
        serde_json::json!({"type":"object","properties":{"pattern":{"type":"string"},"path":{"type":"string"}},"required":["pattern"]})
    );
    async fn execute(
        &self,
        _id: &str,
        args: serde_json::Value,
        _signal: Option<tokio_util::sync::CancellationToken>,
        _update: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        let pattern = required_string(&args, "pattern", "grep")?;
        let root = args
            .get("path")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(".");
        let matches = grep_matches(root, pattern);
        text_result(&matches.join("\n"))
    }
}

fn grep_matches(root: &str, pattern: &str) -> Vec<String> {
    let mut matches = Vec::new();
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        if matches.len() >= 250 {
            break;
        }
        let Ok(content) = std::fs::read_to_string(entry.path()) else {
            continue;
        };
        for (line, text) in content
            .lines()
            .enumerate()
            .filter(|(_, text)| text.contains(pattern))
        {
            matches.push(format!("{}:{}:{}", entry.path().display(), line + 1, text));
        }
    }
    matches
}

#[async_trait::async_trait]
impl AgentTool for GlobTool {
    workspace_tool!(
        "glob",
        "Find files",
        "Find files recursively by a simple glob pattern.",
        serde_json::json!({"type":"object","properties":{"pattern":{"type":"string"},"path":{"type":"string"}},"required":["pattern"]})
    );
    async fn execute(
        &self,
        _id: &str,
        args: serde_json::Value,
        _signal: Option<tokio_util::sync::CancellationToken>,
        _update: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        let pattern = required_string(&args, "pattern", "glob")?;
        let root = args
            .get("path")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(".");
        let mut paths: Vec<_> = walkdir::WalkDir::new(root)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file())
            .filter(|entry| glob_matches(&entry.file_name().to_string_lossy(), pattern))
            .map(|entry| entry.path().display().to_string())
            .collect();
        paths.sort_unstable();
        text_result(&paths.into_iter().take(100).collect::<Vec<_>>().join("\n"))
    }
}

fn required_string<'a>(
    args: &'a serde_json::Value,
    key: &str,
    tool: &str,
) -> Result<&'a str, String> {
    args.get(key)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("{tool} requires string `{key}`"))
}

fn text_result(text: &str) -> Result<AgentToolResult, String> {
    Ok(AgentToolResult {
        content: vec![ToolResultContent::Text { text: text.into() }],
        ..AgentToolResult::default()
    })
}

fn glob_matches(name: &str, pattern: &str) -> bool {
    match pattern.strip_prefix("*") {
        Some(suffix) => name.ends_with(suffix),
        None => name == pattern,
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

    #[tokio::test]
    async fn write_and_edit_preserve_exact_match_safety() {
        let path = std::env::temp_dir().join(format!("runie-edit-{}.txt", std::process::id()));
        WriteFileTool
            .execute(
                "w",
                serde_json::json!({"path": path, "content": "old\nold"}),
                None,
                None,
            )
            .await
            .unwrap();
        let error = EditFileTool
            .execute(
                "e",
                serde_json::json!({"path": path, "old_string": "old", "new_string": "new"}),
                None,
                None,
            )
            .await
            .unwrap_err();
        assert!(error.contains("matched 2"));
        EditFileTool.execute("e", serde_json::json!({"path": path, "old_string": "old", "new_string": "new", "replace_all": true}), None, None).await.unwrap();
        assert_eq!(tokio::fs::read_to_string(&path).await.unwrap(), "new\nnew");
        tokio::fs::remove_file(path).await.unwrap();
    }

    #[tokio::test]
    async fn grep_and_glob_return_deterministic_matches() {
        let root = std::env::temp_dir().join(format!("runie-search-{}", std::process::id()));
        tokio::fs::create_dir_all(&root).await.unwrap();
        tokio::fs::write(root.join("a.rs"), "needle\n")
            .await
            .unwrap();
        tokio::fs::write(root.join("b.txt"), "other\n")
            .await
            .unwrap();
        let grep = GrepTool
            .execute(
                "g",
                serde_json::json!({"path": root, "pattern": "needle"}),
                None,
                None,
            )
            .await
            .unwrap();
        let ToolResultContent::Text { text } = &grep.content[0] else {
            panic!("expected text")
        };
        assert!(text.ends_with(":1:needle"));
        let glob = GlobTool
            .execute(
                "f",
                serde_json::json!({"path": root, "pattern": "*.rs"}),
                None,
                None,
            )
            .await
            .unwrap();
        let ToolResultContent::Text { text } = &glob.content[0] else {
            panic!("expected text")
        };
        assert!(text.ends_with("a.rs"));
        tokio::fs::remove_dir_all(root).await.unwrap();
    }
}
