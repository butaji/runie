//! Find tool — searches for files matching a pattern.

use crate::tool::{which_tool_async, Tool, ToolContext, ToolOutput, ToolStatus};
use anyhow::Result;
use async_trait::async_trait;
use runie_core::path::resolve_path_in;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::time::Instant;
use tokio::process::Command;

/// Input parameters for find tool.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct FindInput {
    /// File pattern to search for
    pub pattern: String,
    /// Root directory to search (default: current directory)
    #[serde(default)]
    pub path: Option<String>,
    /// Maximum number of results (default: 100)
    #[serde(default)]
    pub limit: Option<usize>,
}

pub struct FindTool;

#[async_trait]
impl Tool for FindTool {
    fn name(&self) -> &str { "find" }
    fn description(&self) -> &str { "Find files matching a pattern using fd or find." }
    fn input_schema(&self) -> Value {
        runie_core::tool::generate_schema::<FindInput>()
    }
    fn is_read_only(&self) -> bool { true }
    fn requires_approval(&self, _input: &Value) -> bool { false }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolOutput> {
        let start = Instant::now();
        let typed: FindInput = serde_json::from_value(input)?;
        let path_str = typed.path.as_deref().unwrap_or(".");
        let full_path = resolve_path_in(path_str, &ctx.working_dir);
        let content = run_find(&typed.pattern, &full_path, typed.limit.unwrap_or(100))
            .await
            .unwrap_or_else(|e| format!("Error running find: {}", e));
        let status = determine_find_status(&content);
        Ok(ToolOutput {
            tool_name: "find".to_owned(),
            tool_args: serde_json::json!({ "path": path_str, "pattern": typed.pattern }),
            content,
            bytes_transferred: None,
            duration: start.elapsed(),
            status,
        })
    }
}

fn determine_find_status(content: &str) -> ToolStatus {
    if content.starts_with("Error") || content.is_empty() {
        ToolStatus::Error
    } else {
        ToolStatus::Success
    }
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
