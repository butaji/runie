//! Find tool — searches for files matching a pattern.

use crate::tool::{Tool, ToolContext, ToolOutput, ToolStatus, which_tool_async};
use anyhow::Result;
use async_trait::async_trait;
use runie_core::path::resolve_path_in;
use serde_json::Value;
use std::time::Instant;
use tokio::process::Command;

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
        let full_path = resolve_path_in(&path, &ctx.working_dir);
        let content = run_find(&pattern, &full_path, limit)
            .await
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

async fn run_find(
    pattern: &str,
    path: &std::path::Path,
    limit: usize,
) -> Result<String, std::io::Error> {
    if which_tool_async("fd").await.is_some() {
        run_fd(pattern, path, limit).await
    } else {
        run_find_fallback(pattern, path, limit).await
    }
}

fn determine_find_status(content: &str) -> ToolStatus {
    if content.starts_with("Error") || content.is_empty() {
        ToolStatus::Error
    } else {
        ToolStatus::Success
    }
}

async fn run_fd(
    pattern: &str,
    path: &std::path::Path,
    limit: usize,
) -> Result<String, std::io::Error> {
    let output = Command::new("fd")
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
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        return Ok("No files found matching pattern".to_string());
    }
    Ok(stdout.trim_end().to_string())
}

async fn run_find_fallback(
    pattern: &str,
    path: &std::path::Path,
    limit: usize,
) -> Result<String, std::io::Error> {
    let path_str = path.to_str().unwrap_or(".");
    let output = if pattern.contains('/') {
        Command::new("find")
            .arg(path_str)
            .args(["-maxdepth", "10", "-path", &format!("*/{}", pattern)])
            .output()
            .await?
    } else {
        Command::new("find")
            .arg(path_str)
            .args(["-maxdepth", "10", "-name", pattern])
            .output()
            .await?
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        return Ok("No files found matching pattern".to_string());
    }
    let mut lines: Vec<&str> = stdout.lines().collect();
    lines.truncate(limit);
    Ok(lines.join("\n"))
}
