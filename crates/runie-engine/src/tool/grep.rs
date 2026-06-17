//! Grep tool — searches for patterns in files.

use crate::tool::{Tool, ToolContext, ToolOutput, ToolStatus};
use anyhow::Result;
use async_trait::async_trait;
use runie_core::tool::{resolve_path, tool_error};
use serde_json::Value;
use std::time::Instant;

pub struct GrepTool;

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "grep"
    }

    fn description(&self) -> &str {
        "Search for patterns in files using ripgrep (rg) or grep."
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Search pattern"
                },
                "path": {
                    "type": "string",
                    "description": "Directory or file path to search"
                },
                "glob": {
                    "type": "string",
                    "description": "File glob pattern (e.g., *.rs)"
                },
                "ignore_case": {
                    "type": "boolean",
                    "description": "Case-insensitive search"
                },
                "literal": {
                    "type": "boolean",
                    "description": "Treat pattern as literal string"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of matches (default: 100)"
                }
            },
            "required": ["pattern", "path"]
        })
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn requires_approval(&self, _input: &Value) -> bool {
        false
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolOutput> {
        let start = Instant::now();
        let (pattern, path, glob, ignore_case, literal, limit) = parse_grep_input(&input)?;
        let full_path = resolve_path(&path, &ctx.working_dir);
        run_grep_impl(
            &pattern,
            &full_path,
            glob,
            ignore_case,
            literal,
            limit,
            start,
        )
    }
}

fn parse_grep_input(input: &Value) -> Result<(String, String, Option<String>, bool, bool, usize)> {
    let pattern = input["pattern"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("pattern is required"))?
        .to_string();
    let path = input["path"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("path is required"))?
        .to_string();
    let glob = input["glob"].as_str().map(String::from);
    let ignore_case = input["ignore_case"].as_bool().unwrap_or(false);
    let literal = input["literal"].as_bool().unwrap_or(false);
    let limit = input["limit"].as_u64().unwrap_or(100) as usize;
    Ok((pattern, path, glob, ignore_case, literal, limit))
}

fn run_grep_impl(
    pattern: &str,
    path: &std::path::Path,
    glob: Option<String>,
    ignore_case: bool,
    literal: bool,
    limit: usize,
    start: Instant,
) -> Result<ToolOutput> {
    let tool = select_grep_tool();
    let args = build_grep_args(pattern, path, glob.as_deref(), ignore_case, literal, limit);
    let output = match run_grep_command(tool, &args) {
        Ok(o) => o,
        Err(e) => return Ok(tool_error("grep", &format!("Error running grep: {}", e), start, false)),
    };
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let (content, status) = parse_grep_output(&stdout, &stderr, output.status.code());
    Ok(build_grep_output(pattern, path, content, status, stdout.len(), start))
}

fn select_grep_tool() -> &'static str {
    if crate::tool::which_tool("rg").is_some() {
        "rg"
    } else {
        "grep"
    }
}

fn run_grep_command(tool: &str, args: &[String]) -> Result<std::process::Output, std::io::Error> {
    std::process::Command::new(tool).args(args).output()
}

fn build_grep_output(
    pattern: &str,
    path: &std::path::Path,
    content: String,
    status: ToolStatus,
    bytes: usize,
    start: Instant,
) -> ToolOutput {
    ToolOutput {
        tool_name: "grep".to_string(),
        tool_args: serde_json::json!({ "path": path, "pattern": pattern }),
        content,
        bytes_transferred: Some(bytes as u64),
        duration: start.elapsed(),
        status,
    }
}

fn parse_grep_output(stdout: &str, stderr: &str, code: Option<i32>) -> (String, ToolStatus) {
    if stdout.trim().is_empty() {
        if code == Some(1) {
            ("No matches found".to_string(), ToolStatus::Success)
        } else {
            (format!("Error: {}", stderr.trim()), ToolStatus::Error)
        }
    } else {
        (stdout.to_string(), ToolStatus::Success)
    }
}

fn build_grep_args(
    pattern: &str,
    path: &std::path::Path,
    glob: Option<&str>,
    ignore_case: bool,
    literal: bool,
    limit: usize,
) -> Vec<String> {
    let mut args = vec![
        "--line-number".into(),
        "--color=never".into(),
        "--hidden".into(),
    ];
    if ignore_case {
        args.push("--ignore-case".into());
    }
    if literal {
        args.push("--fixed-strings".into());
    }
    if let Some(g) = glob {
        args.push("--glob".into());
        args.push(g.into());
    }
    args.push("--max-count".into());
    args.push(limit.to_string());
    args.push("--".into());
    args.push(pattern.into());
    args.push(path.to_string_lossy().to_string());
    args
}
