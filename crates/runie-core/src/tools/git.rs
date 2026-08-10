//! Safe, read-only Git inspection tools. Mutations stay behind explicit tools.

use crate::types::{AgentTool, AgentToolResult, ToolResultContent};

const GIT_OUTPUT_MAX_BYTES: usize = 100 * 1024;
#[path = "git_conflicts.rs"]
mod conflicts;
pub use conflicts::{
    begin_conflict_recovery, classify_conflicts, plan_conflict_recovery, reduce_conflict_recovery,
    GitConflictAction, GitConflictRecoveryEvent, GitConflictRecoveryPlan, GitConflictRecoveryState,
    GitConflictRecoveryStatus, GitConflictSummary,
};

macro_rules! git_tool_types { ($($name:ident),+ $(,)?) => { $(#[derive(Default)] pub struct $name;)+ }; }
git_tool_types!(
    GitStatusTool,
    GitDiffTool,
    GitReviewTool,
    GitWorktreeTool,
    GitCommitPrepareTool,
    GitCommitTool,
    GitPushTool,
    GitRevertTool
);

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GitCommitPrepareRequest {
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GitPushRequest {
    pub remote: String,
    pub reference: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GitRevertRequest {
    pub commit: String,
}

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

#[async_trait::async_trait]
impl AgentTool for GitWorktreeTool {
    fn name(&self) -> &str {
        "git_worktree"
    }
    fn label(&self) -> &str {
        "Git worktrees"
    }
    fn description(&self) -> &str {
        "List Git worktrees and their branch and commit identities."
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
        git_result(&["worktree", "list", "--porcelain"], signal).await
    }
}

#[async_trait::async_trait]
impl AgentTool for GitCommitPrepareTool {
    fn name(&self) -> &str {
        "git_commit_prepare"
    }
    fn label(&self) -> &str {
        "Prepare Git commit"
    }
    fn description(&self) -> &str {
        "Validate a commit message and summarize a proposed commit without mutating Git."
    }
    fn parameters(&self) -> Option<serde_json::Value> {
        Some(
            serde_json::json!({"type":"object","properties":{"message":{"type":"string","minLength":1}},"required":["message"]}),
        )
    }
    fn validate_arguments(&self, args: &serde_json::Value) -> Result<(), String> {
        let request: GitCommitPrepareRequest = serde_json::from_value(args.clone())
            .map_err(|error| format!("invalid commit preparation: {error}"))?;
        if request.message.trim().is_empty() {
            return Err("commit message must not be empty".into());
        }
        if request
            .message
            .lines()
            .next()
            .unwrap_or_default()
            .chars()
            .count()
            > 72
        {
            return Err("commit subject must be at most 72 characters".into());
        }
        Ok(())
    }
    async fn execute(
        &self,
        _: &str,
        args: serde_json::Value,
        signal: Option<tokio_util::sync::CancellationToken>,
        _: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        self.validate_arguments(&args)?;
        let request: GitCommitPrepareRequest =
            serde_json::from_value(args).map_err(|error| error.to_string())?;
        let status = git_capture(&["status", "--short"], signal.clone()).await?;
        let diff = git_capture(&["diff", "--stat"], signal).await?;
        if !status.status.success() || !diff.status.success() {
            return Err("unable to inspect Git changes".into());
        }
        let status_text = String::from_utf8_lossy(&status.stdout).into_owned();
        let stat_text = String::from_utf8_lossy(&diff.stdout).into_owned();
        let changed = !status_text.trim().is_empty();
        Ok(AgentToolResult {
            content: vec![ToolResultContent::Text {
                text: format!(
                    "Commit proposal: {}\n{}",
                    request.message,
                    if changed {
                        stat_text.clone()
                    } else {
                        "No changed files".into()
                    }
                ),
            }],
            details: serde_json::json!({"message": request.message, "changed": changed, "status": status_text, "stat": stat_text, "mutated": false}),
            ..Default::default()
        })
    }
}

#[async_trait::async_trait]
impl AgentTool for GitCommitTool {
    fn name(&self) -> &str {
        "git_commit"
    }
    fn label(&self) -> &str {
        "Create Git commit"
    }
    fn description(&self) -> &str {
        "Create a Git commit after approval allows this mutation."
    }
    fn resource_key(&self, _args: &serde_json::Value) -> Option<String> {
        Some("git:index".into())
    }
    fn parameters(&self) -> Option<serde_json::Value> {
        Some(
            serde_json::json!({"type":"object","properties":{"message":{"type":"string","minLength":1}},"required":["message"]}),
        )
    }
    fn validate_arguments(&self, args: &serde_json::Value) -> Result<(), String> {
        let request: GitCommitPrepareRequest = serde_json::from_value(args.clone())
            .map_err(|error| format!("invalid Git commit: {error}"))?;
        if request.message.trim().is_empty() {
            return Err("commit message must not be empty".into());
        }
        if request
            .message
            .lines()
            .next()
            .unwrap_or_default()
            .chars()
            .count()
            > 72
        {
            return Err("commit subject must be at most 72 characters".into());
        }
        Ok(())
    }
    async fn execute(
        &self,
        _: &str,
        args: serde_json::Value,
        signal: Option<tokio_util::sync::CancellationToken>,
        _: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        self.validate_arguments(&args)?;
        let request: GitCommitPrepareRequest =
            serde_json::from_value(args).map_err(|error| error.to_string())?;
        let output = git_result(&["commit", "-m", &request.message], signal).await?;
        Ok(AgentToolResult {
            details: serde_json::json!({"message": request.message, "mutated": true, "output": output.details}),
            ..output
        })
    }
}

#[async_trait::async_trait]
impl AgentTool for GitPushTool {
    fn name(&self) -> &str {
        "git_push"
    }
    fn label(&self) -> &str {
        "Push Git ref"
    }
    fn description(&self) -> &str {
        "Push an explicit Git ref after approval allows this mutation."
    }
    fn resource_key(&self, _: &serde_json::Value) -> Option<String> {
        Some("git:remote".into())
    }
    fn parameters(&self) -> Option<serde_json::Value> {
        Some(
            serde_json::json!({"type":"object","properties":{"remote":{"type":"string","minLength":1},"reference":{"type":"string","minLength":1}},"required":["remote","reference"]}),
        )
    }
    fn validate_arguments(&self, args: &serde_json::Value) -> Result<(), String> {
        let request: GitPushRequest = serde_json::from_value(args.clone())
            .map_err(|error| format!("invalid Git push: {error}"))?;
        validate_git_token("remote", &request.remote)?;
        validate_git_token("reference", &request.reference)
    }
    async fn execute(
        &self,
        _: &str,
        args: serde_json::Value,
        signal: Option<tokio_util::sync::CancellationToken>,
        _: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        self.validate_arguments(&args)?;
        let request: GitPushRequest =
            serde_json::from_value(args).map_err(|error| error.to_string())?;
        let output = git_result(&["push", &request.remote, &request.reference], signal).await?;
        Ok(AgentToolResult {
            details: serde_json::json!({"remote": request.remote, "reference": request.reference, "mutated": true, "output": output.details}),
            ..output
        })
    }
}

#[async_trait::async_trait]
impl AgentTool for GitRevertTool {
    fn name(&self) -> &str {
        "git_revert"
    }
    fn label(&self) -> &str {
        "Revert Git commit"
    }
    fn description(&self) -> &str {
        "Create an inverse commit for an explicit Git commit after approval."
    }
    fn resource_key(&self, _: &serde_json::Value) -> Option<String> {
        Some("git:index".into())
    }
    fn parameters(&self) -> Option<serde_json::Value> {
        Some(
            serde_json::json!({"type":"object","properties":{"commit":{"type":"string","minLength":7}},"required":["commit"]}),
        )
    }
    fn validate_arguments(&self, args: &serde_json::Value) -> Result<(), String> {
        let request: GitRevertRequest = serde_json::from_value(args.clone())
            .map_err(|error| format!("invalid Git revert: {error}"))?;
        validate_commit_reference(&request.commit)
    }
    async fn execute(
        &self,
        _: &str,
        args: serde_json::Value,
        signal: Option<tokio_util::sync::CancellationToken>,
        _: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        self.validate_arguments(&args)?;
        let request: GitRevertRequest =
            serde_json::from_value(args).map_err(|error| error.to_string())?;
        let output = git_result(&["revert", "--no-edit", &request.commit], signal).await?;
        Ok(AgentToolResult {
            details: serde_json::json!({"commit": request.commit, "mutated": true, "output": output.details}),
            ..output
        })
    }
}

fn validate_commit_reference(value: &str) -> Result<(), String> {
    if value.trim().is_empty() || value.chars().any(char::is_whitespace) || value.starts_with('-') {
        return Err(
            "commit must be a nonempty Git reference without whitespace or a leading dash".into(),
        );
    }
    if !value.chars().all(|character| character.is_ascii_hexdigit()) || value.len() < 7 {
        return Err("commit must be at least 7 hexadecimal characters".into());
    }
    Ok(())
}

fn validate_git_token(name: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() || value.chars().any(char::is_whitespace) || value.starts_with('-') {
        return Err(format!(
            "{name} must be a nonempty Git token without whitespace or a leading dash"
        ));
    }
    Ok(())
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
#[path = "git_tests.rs"]
mod tests;
