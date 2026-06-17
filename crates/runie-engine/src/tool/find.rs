//! Find tool — searches for files matching a pattern.

use crate::tool::{Tool, ToolContext, ToolOutput, ToolStatus};
use anyhow::Result;
use async_trait::async_trait;
use runie_core::tool::resolve_path;
use serde_json::Value;
use std::time::Instant;

pub struct FindTool;

#[async_trait]
impl Tool for FindTool {
    fn name(&self) -> &str {
        "find"
    }

    fn description(&self) -> &str {
        "Find files matching a pattern using fd or find."
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "File pattern to search for"
                },
                "path": {
                    "type": "string",
                    "description": "Root directory to search (default: current directory)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of results (default: 100)"
                }
            },
            "required": ["pattern"]
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
        let (pattern, path, limit) = parse_find_input(&input)?;
        let full_path = resolve_path(&path, &ctx.working_dir);
        let content = run_find(&pattern, &full_path, limit)
            .unwrap_or_else(|e| format!("Error running find: {}", e));
        let status = determine_find_status(&content);
        Ok(ToolOutput {
            tool_name: "find".to_string(),
            tool_args: serde_json::json!({ "path": path, "pattern": pattern }),
            content,
            bytes_transferred: None,
            duration: start.elapsed(),
            status,
        })
    }
}

fn parse_find_input(input: &Value) -> Result<(String, String, usize)> {
    let pattern = input["pattern"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("pattern is required"))?
        .to_string();
    let path = input["path"].as_str().unwrap_or(".").to_string();
    let limit = input["limit"].as_u64().unwrap_or(100) as usize;
    Ok((pattern, path, limit))
}

fn run_find(pattern: &str, path: &std::path::Path, limit: usize) -> Result<String, std::io::Error> {
    if crate::tool::which_tool("fd").is_some() {
        run_fd(pattern, path, limit)
    } else {
        run_find_fallback(pattern, path, limit)
    }
}

fn determine_find_status(content: &str) -> ToolStatus {
    if content.starts_with("Error") || content.is_empty() {
        ToolStatus::Error
    } else {
        ToolStatus::Success
    }
}

fn run_fd(pattern: &str, path: &std::path::Path, limit: usize) -> Result<String, std::io::Error> {
    let output = std::process::Command::new("fd")
        .args([
            "--glob",
            "--color=never",
            "--hidden",
            "--no-require-git",
            "--max-results",
            &limit.to_string(),
            pattern,
            path.to_str().unwrap_or("."),
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        return Ok("No files found matching pattern".to_string());
    }
    Ok(stdout.trim_end().to_string())
}

fn run_find_fallback(
    pattern: &str,
    path: &std::path::Path,
    limit: usize,
) -> Result<String, std::io::Error> {
    let path_str = path.to_str().unwrap_or(".");
    let output = if pattern.contains('/') {
        std::process::Command::new("find")
            .arg(path_str)
            .args(["-maxdepth", "10", "-path", &format!("*/{}", pattern)])
            .output()?
    } else {
        std::process::Command::new("find")
            .arg(path_str)
            .args(["-maxdepth", "10", "-name", pattern])
            .output()?
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        return Ok("No files found matching pattern".to_string());
    }
    let mut lines: Vec<&str> = stdout.lines().collect();
    lines.truncate(limit);
    Ok(lines.join("\n"))
}
