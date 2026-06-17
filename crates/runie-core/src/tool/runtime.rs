//! Unified `ToolRuntime` trait and supporting approval/sandbox types.
//!
//! Provides a consistent async interface for all tool execution with hooks for
//! approval requirements and future sandboxing strategies.

use std::fmt;

use async_trait::async_trait;

use super::{ToolContext, ToolOutput};

/// Approval level required before a tool may execute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecApprovalRequirement {
    /// No approval needed.
    None,
    /// Read-only operation; inform the user but do not block.
    Inform,
    /// Requires explicit user approval.
    Prompt,
    /// Operation is blocked by policy.
    Blocked,
}

impl ExecApprovalRequirement {
    /// Returns true when the caller must obtain explicit approval.
    pub fn requires_approval(self) -> bool {
        matches!(self, Self::Prompt)
    }
}

/// Description of a network access request for approval purposes.
#[derive(Debug, Clone)]
pub struct NetworkApprovalSpec {
    pub host: String,
    pub port: Option<u16>,
}

/// Record of a sandboxing decision for a tool execution attempt.
#[derive(Debug, Clone, Default)]
pub struct SandboxAttempt {
    pub allowed: bool,
    pub sandbox_type: Option<String>,
}

/// Errors that can occur during tool runtime execution.
#[derive(Debug)]
pub enum ToolError {
    Execution(String),
    ApprovalRequired(ExecApprovalRequirement),
    SandboxBlocked(String),
    Io(std::io::Error),
    Serde(serde_json::Error),
}

impl fmt::Display for ToolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Execution(msg) => write!(f, "execution failed: {}", msg),
            Self::ApprovalRequired(req) => write!(f, "approval required: {:?}", req),
            Self::SandboxBlocked(msg) => write!(f, "sandbox blocked: {}", msg),
            Self::Io(e) => write!(f, "io error: {}", e),
            Self::Serde(e) => write!(f, "serde error: {}", e),
        }
    }
}

impl std::error::Error for ToolError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Serde(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for ToolError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<serde_json::Error> for ToolError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serde(err)
    }
}

/// Unified async interface for tool execution.
#[async_trait]
pub trait ToolRuntime: Send + Sync {
    /// Tool name used for logging and registry lookups.
    fn name(&self) -> &str;

    /// Approval level required for this tool invocation.
    fn exec_approval_requirement(&self) -> ExecApprovalRequirement;

    /// Optional network access specification for approval.
    fn network_approval_spec(&self, _ctx: &ToolContext) -> Option<NetworkApprovalSpec> {
        None
    }

    /// Execute the tool and return structured output.
    async fn run(&self, ctx: &ToolContext) -> Result<ToolOutput, ToolError>;

    /// Convenience helper: true when explicit approval is required.
    fn requires_approval(&self) -> bool {
        self.exec_approval_requirement().requires_approval()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use serde_json::Value;

    use super::*;
    use crate::tool::ToolStatus;

    struct OkTool;
    #[async_trait]
    impl ToolRuntime for OkTool {
        fn name(&self) -> &str {
            "ok_tool"
        }
        fn exec_approval_requirement(&self) -> ExecApprovalRequirement {
            ExecApprovalRequirement::None
        }
        async fn run(&self, _ctx: &ToolContext) -> Result<ToolOutput, ToolError> {
            Ok(ToolOutput {
                tool_name: "ok_tool".to_string(),
                tool_args: Value::Null,
                content: "ok".to_string(),
                bytes_transferred: None,
                duration: Duration::default(),
                status: ToolStatus::Success,
            })
        }
    }

    struct ErrTool;
    #[async_trait]
    impl ToolRuntime for ErrTool {
        fn name(&self) -> &str {
            "err_tool"
        }
        fn exec_approval_requirement(&self) -> ExecApprovalRequirement {
            ExecApprovalRequirement::Prompt
        }
        async fn run(&self, _ctx: &ToolContext) -> Result<ToolOutput, ToolError> {
            Err(ToolError::Execution("boom".to_string()))
        }
    }

    #[test]
    fn tool_runtime_exec_approval_requirement() {
        assert_eq!(
            OkTool.exec_approval_requirement(),
            ExecApprovalRequirement::None
        );
        assert_eq!(
            ErrTool.exec_approval_requirement(),
            ExecApprovalRequirement::Prompt
        );
    }

    #[tokio::test]
    async fn tool_runtime_run_returns_output() {
        let ctx = ToolContext::default();
        let out = OkTool.run(&ctx).await.unwrap();
        assert_eq!(out.content, "ok");
        assert_eq!(out.status, ToolStatus::Success);
    }

    #[tokio::test]
    async fn tool_runtime_run_returns_error() {
        let ctx = ToolContext::default();
        let err = ErrTool.run(&ctx).await.unwrap_err();
        assert!(matches!(err, ToolError::Execution(ref s) if s == "boom"));
    }
}
