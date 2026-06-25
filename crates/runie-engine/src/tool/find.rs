//! Find tool — searches for files matching a pattern.

use crate::define_tool;
use crate::tool::{which_tool_async, Tool, ToolContext, ToolOutput, ToolStatus};
use anyhow::Result;
use async_trait::async_trait;
use runie_core::path::resolve_path_in;
use serde_json::Value;
use std::time::Instant;
use tokio::process::Command;

pub struct FindTool;

#[allow(clippy::use_self)]
#[async_trait]
impl Tool for FindTool {
    define_tool! {
        name: "find",
        description: "Find files matching a pattern using fd or find.",
        read_only: true,
        approval: false,
        fields: {
            "pattern": ("string", "File pattern to search for"),
            "path": ("string", "Root directory to search (default: current directory)"),
            "limit": ("integer", "Maximum number of results (default: 100)")
        },
        required: ["pattern"]
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
            tool_name: "find".to_owned(),
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
        .ok_or_else(|| anyhow::anyhow!("pattern is required"))?.to_owned();
    let path = input["path"].as_str().unwrap_or(".").to_owned();
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
        return Ok("No files found matching pattern".to_owned());
    }
    Ok(stdout.trim_end().to_owned())
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
        return Ok("No files found matching pattern".to_owned());
    }
    let mut lines: Vec<&str> = stdout.lines().collect();
    lines.truncate(limit);
    Ok(lines.join("\n"))
}
