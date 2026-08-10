//! Typed boundary for owned background shell jobs.

use crate::tools::executor::ToolExecContext;
use crate::types::ToolCall;
use crate::types::{AgentTool, AgentToolResult, ToolResultContent};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BackgroundShellRequest {
    pub command: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BackgroundCancelRequest {
    pub job_id: String,
}

#[derive(Default)]
pub struct BackgroundShellTool;

#[derive(Default)]
pub struct BackgroundJobsTool;

#[derive(Default)]
pub struct BackgroundCancelTool;

#[async_trait::async_trait]
impl AgentTool for BackgroundShellTool {
    fn name(&self) -> &str {
        "background_bash"
    }
    fn label(&self) -> &str {
        "Background shell"
    }
    fn description(&self) -> &str {
        "Start a shell command owned by the background job actor."
    }
    fn parameters(&self) -> Option<serde_json::Value> {
        Some(
            serde_json::json!({"type":"object","properties":{"command":{"type":"string","minLength":1}},"required":["command"]}),
        )
    }
    fn validate_arguments(&self, args: &serde_json::Value) -> Result<(), String> {
        let request: BackgroundShellRequest = serde_json::from_value(args.clone())
            .map_err(|error| format!("invalid background shell request: {error}"))?;
        if request.command.trim().is_empty() {
            return Err("command must not be empty".into());
        }
        Ok(())
    }
    async fn execute(
        &self,
        _: &str,
        _: serde_json::Value,
        _: Option<tokio_util::sync::CancellationToken>,
        _: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        Err("background_bash requires an owning background hook".into())
    }
}

#[async_trait::async_trait]
impl AgentTool for BackgroundJobsTool {
    fn name(&self) -> &str {
        "background_jobs"
    }
    fn label(&self) -> &str {
        "Background jobs"
    }
    fn description(&self) -> &str {
        "Inspect owned background shell jobs and their current output."
    }
    fn parameters(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({"type":"object"}))
    }
    async fn execute(
        &self,
        _: &str,
        _: serde_json::Value,
        _: Option<tokio_util::sync::CancellationToken>,
        _: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        Err("background_jobs requires an owning background jobs hook".into())
    }
}

#[async_trait::async_trait]
impl AgentTool for BackgroundCancelTool {
    fn name(&self) -> &str {
        "background_cancel"
    }
    fn label(&self) -> &str {
        "Cancel background job"
    }
    fn description(&self) -> &str {
        "Cancel a running owned background shell job."
    }
    fn parameters(&self) -> Option<serde_json::Value> {
        Some(
            serde_json::json!({"type":"object","properties":{"job_id":{"type":"string","minLength":1}},"required":["job_id"]}),
        )
    }
    fn validate_arguments(&self, args: &serde_json::Value) -> Result<(), String> {
        let request: BackgroundCancelRequest = serde_json::from_value(args.clone())
            .map_err(|error| format!("invalid background cancellation: {error}"))?;
        if request.job_id.trim().is_empty() {
            return Err("job_id must not be empty".into());
        }
        Ok(())
    }
    async fn execute(
        &self,
        _: &str,
        _: serde_json::Value,
        _: Option<tokio_util::sync::CancellationToken>,
        _: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        Err("background_cancel requires an owning background cancellation hook".into())
    }
}

pub(crate) fn result(value: serde_json::Value) -> AgentToolResult {
    AgentToolResult {
        content: vec![ToolResultContent::Text {
            text: value.to_string(),
        }],
        details: value,
        ..Default::default()
    }
}

pub(crate) async fn execute_shell(
    call: &ToolCall,
    ctx: &ToolExecContext,
) -> Result<AgentToolResult, String> {
    let Some(hook) = &ctx.hooks.background_shell else {
        return Err("background_bash requires an owning background hook".into());
    };
    let request = serde_json::from_value(call.arguments.clone())
        .map_err(|error| format!("invalid background shell request: {error}"))?;
    Ok(result(hook(request).await?))
}

pub(crate) async fn execute_jobs(ctx: &ToolExecContext) -> Result<AgentToolResult, String> {
    let Some(hook) = &ctx.hooks.background_jobs else {
        return Err("background_jobs requires an owning background jobs hook".into());
    };
    Ok(result(hook().await?))
}

pub(crate) async fn execute_cancel(
    call: &ToolCall,
    ctx: &ToolExecContext,
) -> Result<AgentToolResult, String> {
    let Some(hook) = &ctx.hooks.background_cancel else {
        return Err("background_cancel requires an owning background cancellation hook".into());
    };
    let request = serde_json::from_value(call.arguments.clone())
        .map_err(|error| format!("invalid background cancellation: {error}"))?;
    Ok(result(hook(request).await?))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_non_empty_command() {
        let tool = BackgroundShellTool;
        assert!(tool
            .validate_arguments(&serde_json::json!({"command":"printf ok"}))
            .is_ok());
        assert!(tool
            .validate_arguments(&serde_json::json!({"command":" "}))
            .is_err());
    }

    #[test]
    fn jobs_tool_is_read_only_and_has_no_arguments() {
        let tool = BackgroundJobsTool;
        assert_eq!(tool.name(), "background_jobs");
        assert_eq!(
            tool.parameters(),
            Some(serde_json::json!({"type":"object"}))
        );
    }

    #[test]
    fn cancellation_requires_a_job_id() {
        let tool = BackgroundCancelTool;
        assert!(tool
            .validate_arguments(&serde_json::json!({"job_id":"0"}))
            .is_ok());
        assert!(tool
            .validate_arguments(&serde_json::json!({"job_id":" "}))
            .is_err());
    }
}
