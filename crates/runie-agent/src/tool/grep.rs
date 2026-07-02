//! Grep tool — searches for patterns in files.

use crate::tool::constants::GREP_DEFAULT_LIMIT;
use crate::tool::which_tool_async;
use crate::tool::{ToolContext, ToolOutput, ToolStatus};
use runie_core::path::resolve_path_in;
use runie_core::tool::{tool_error, ToolDef};
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::time::Instant;
use tokio::process::Command;

/// Input parameters for grep tool.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GrepInput {
    /// Search pattern
    pub pattern: String,
    /// Directory or file path to search
    pub path: String,
    /// File glob pattern (e.g., *.rs)
    #[serde(default)]
    pub glob: Option<String>,
    /// Case-insensitive search
    #[serde(default)]
    pub ignore_case: Option<bool>,
    /// Treat pattern as literal string
    #[serde(default)]
    pub literal: Option<bool>,
    /// Maximum number of matches (default: 100)
    #[serde(default)]
    pub limit: Option<usize>,
}

pub struct GrepTool;

impl ToolDef for GrepTool {
    type Input = GrepInput;

    const NAME: &'static str = "grep";
    const DESCRIPTION: &'static str = "Search for patterns in files using ripgrep (rg) or grep.";
    const READ_ONLY: bool = true;
    const REQUIRES_APPROVAL: bool = false;

    async fn execute(input: Self::Input, ctx: &ToolContext) -> ToolOutput {
        let start = Instant::now();
        let full_path = resolve_path_in(&input.path, &ctx.working_dir);
        run_grep_impl(
            &input.pattern,
            &full_path,
            input.glob.as_deref(),
            input.ignore_case.unwrap_or(false),
            input.literal.unwrap_or(false),
            input.limit.unwrap_or(GREP_DEFAULT_LIMIT),
            start,
        )
        .await
    }
}

async fn run_grep_impl(
    pattern: &str,
    path: &std::path::Path,
    glob: Option<&str>,
    ignore_case: bool,
    literal: bool,
    limit: usize,
    start: Instant,
) -> ToolOutput {
    let tool = select_grep_tool().await;
    let args = build_grep_args(pattern, path, glob, ignore_case, literal, limit);
    let output = match run_grep_command(tool, &args).await {
        Ok(o) => o,
        Err(e) => return tool_error("grep", &format!("Error running grep: {}", e), start, false),
    };
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let (content, status) = parse_grep_output(&stdout, &stderr, output.status.code());
    build_grep_output(pattern, path, content, status, stdout.len(), start)
}

async fn select_grep_tool() -> &'static str {
    if which_tool_async("rg").await.is_some() {
        "rg"
    } else {
        "grep"
    }
}

async fn run_grep_command(
    tool: &str,
    args: &[String],
) -> Result<std::process::Output, std::io::Error> {
    Command::new(tool).args(args).output().await
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
        tool_name: "grep".to_owned(),
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
            ("No matches found".to_owned(), ToolStatus::Success)
        } else {
            (format!("Error: {}", stderr.trim()), ToolStatus::Error)
        }
    } else {
        (stdout.to_owned(), ToolStatus::Success)
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
