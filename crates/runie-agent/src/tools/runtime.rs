//! `ToolRuntime` implementation for the built-in agent tool enum.

use async_trait::async_trait;
use runie_core::tool::runtime::{
    ExecApprovalRequirement, NetworkApprovalSpec, ToolError, ToolRuntime,
};
use runie_core::tool::{ToolContext, ToolOutput, ToolStatus};

use super::Tool;

#[async_trait]
impl ToolRuntime for Tool {
    fn name(&self) -> &str {
        self.name()
    }

    fn exec_approval_requirement(&self) -> ExecApprovalRequirement {
        if self.is_read_only() {
            ExecApprovalRequirement::None
        } else {
            ExecApprovalRequirement::Prompt
        }
    }

    fn network_approval_spec(&self, _ctx: &ToolContext) -> Option<NetworkApprovalSpec> {
        None
    }

    async fn run(&self, _ctx: &ToolContext) -> Result<ToolOutput, ToolError> {
        let result = self.execute_with_policy(&crate::truncate::TruncationPolicy::default());
        runtime_output(result.output)
    }
}

fn runtime_output(output: ToolOutput) -> Result<ToolOutput, ToolError> {
    match output.status {
        ToolStatus::Success => Ok(output),
        ToolStatus::Error => Err(ToolError::Execution(output.content)),
        ToolStatus::TimedOut => Err(ToolError::Execution("command timed out".to_string())),
        ToolStatus::Blocked => Err(ToolError::SandboxBlocked(output.content)),
        ToolStatus::AwaitingUser => Err(ToolError::ApprovalRequired(
            ExecApprovalRequirement::Prompt,
        )),
    }
}
