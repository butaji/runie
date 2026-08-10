//! Safe, read-only Git inspection tools. Mutations stay behind explicit tools.

use crate::types::{AgentTool, AgentToolResult, ToolResultContent};

const GIT_OUTPUT_MAX_BYTES: usize = 100 * 1024;

macro_rules! git_tool_types { ($($name:ident),+ $(,)?) => { $(#[derive(Default)] pub struct $name;)+ }; }
git_tool_types!(GitStatusTool, GitDiffTool, GitReviewTool);

#[async_trait::async_trait]
impl AgentTool for GitStatusTool {
    fn name(&self) -> &str {
        "git_status"
    }
    fn label(&self) -> &str {
        "Git status"
    }
    fn description(&self) -> &str {
        "Inspect the current Git branch and changed files."
    }
    fn parameters(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({"type":"object"}))
    }
    async fn execute(
        &self,
        _: &str,
        _: serde_json::Value,
        signal: Option<tokio_util::sync::CancellationToken>,
        _: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        git_result(&["status", "--short", "--branch"], signal).await
    }
}

#[async_trait::async_trait]
impl AgentTool for GitDiffTool {
    fn name(&self) -> &str {
        "git_diff"
    }
    fn label(&self) -> &str {
        "Git diff"
    }
    fn description(&self) -> &str {
        "Inspect the unstaged Git diff without changing files."
    }
    fn parameters(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({"type":"object","properties":{"stat":{"type":"boolean"}}}))
    }
    async fn execute(
        &self,
        _: &str,
        args: serde_json::Value,
        signal: Option<tokio_util::sync::CancellationToken>,
        _: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        let command = if args
            .get("stat")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
        {
            vec!["diff", "--stat"]
        } else {
            vec!["diff", "--no-ext-diff", "--unified=3"]
        };
        git_result(&command, signal).await
    }
}

#[async_trait::async_trait]
impl AgentTool for GitReviewTool {
    fn name(&self) -> &str {
        "git_review"
    }
    fn label(&self) -> &str {
        "Review Git patch"
    }
    fn description(&self) -> &str {
        "Check the unstaged patch for whitespace errors and summarize changed files."
    }
    fn parameters(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({"type":"object"}))
    }
    async fn execute(
        &self,
        _: &str,
        _: serde_json::Value,
        signal: Option<tokio_util::sync::CancellationToken>,
        _: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        let check = git_capture(&["diff", "--check"], signal.clone()).await?;
        let stat = git_capture(&["diff", "--stat"], signal).await?;
        let clean = check.status.success();
        let text = if clean {
            format!(
                "Patch review clean\n{}",
                String::from_utf8_lossy(&stat.stdout)
            )
        } else {
            format!(
                "Patch review found whitespace errors\n{}",
                String::from_utf8_lossy(&check.stdout)
            )
        };
        Ok(AgentToolResult {
            content: vec![ToolResultContent::Text { text }],
            details: serde_json::json!({"clean": clean, "stat": String::from_utf8_lossy(&stat.stdout), "whitespace_errors": String::from_utf8_lossy(&check.stdout)}),
            ..Default::default()
        })
    }
}

async fn git_result(
    args: &[&str],
    signal: Option<tokio_util::sync::CancellationToken>,
) -> Result<AgentToolResult, String> {
    let mut command = tokio::process::Command::new("git");
    command
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let output = git_capture(args, signal).await?;
    if !output.status.success() {
        return Err(format!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    let truncated = text.len() > GIT_OUTPUT_MAX_BYTES;
    text.truncate(GIT_OUTPUT_MAX_BYTES);
    Ok(AgentToolResult {
        content: vec![ToolResultContent::Text { text }],
        details: serde_json::json!({"command": args, "truncated": truncated}),
        ..Default::default()
    })
}

async fn git_capture(
    args: &[&str],
    signal: Option<tokio_util::sync::CancellationToken>,
) -> Result<std::process::Output, String> {
    let mut command = tokio::process::Command::new("git");
    command
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let child = command.spawn().map_err(|error| format!("git: {error}"))?;
    if let Some(signal) = signal {
        tokio::select! { result = child.wait_with_output() => result, _ = signal.cancelled() => return Err("git inspection cancelled".into()) }
            .map_err(|error| format!("git: {error}"))
    } else {
        child
            .wait_with_output()
            .await
            .map_err(|error| format!("git: {error}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn status_is_read_only_and_returns_structured_metadata() {
        let result = GitStatusTool
            .execute("1", serde_json::json!({}), None, None)
            .await
            .unwrap();
        assert_eq!(result.details["command"][0], "status");
        assert!(!result.details["truncated"].as_bool().unwrap());
    }
    #[tokio::test]
    async fn diff_stat_is_a_valid_projection() {
        let result = GitDiffTool
            .execute("1", serde_json::json!({"stat":true}), None, None)
            .await
            .unwrap();
        assert_eq!(result.details["command"][0], "diff");
    }
    #[tokio::test]
    async fn review_reports_a_machine_readable_clean_state() {
        let result = GitReviewTool
            .execute("1", serde_json::json!({}), None, None)
            .await
            .unwrap();
        assert!(result.details["clean"].is_boolean());
        assert!(result.details["stat"].is_string());
    }
}
