//! `cancel_subagent` orchestrator tool.

use std::time::Instant;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::Value;

use crate::tool::{Tool, ToolContext, ToolOutput, ToolStatus};

/// Cancels a running subagent.
#[derive(Debug, Clone, Copy, Default)]
pub struct CancelSubagentTool;

impl CancelSubagentTool {
    pub fn execute(&self, input: Value, ctx: &ToolContext) -> Result<ToolOutput> {
        let start = Instant::now();
        let agent_id = input["agent_id"]
            .as_str()
            .ok_or_else(|| anyhow!("cancel_subagent: missing agent_id"))?
            .to_string();
        let registry = ctx
            .agent_registry
            .as_ref()
            .ok_or_else(|| anyhow!("cancel_subagent: no agent registry in context"))?;
        let mut guard = registry.lock().map_err(|e| anyhow!("lock poisoned: {e}"))?;
        guard.close(agent_id.clone()).map_err(|e| anyhow!("{e}"))?;
        Ok(ToolOutput {
            tool_name: "cancel_subagent".to_string(),
            tool_args: input,
            content: format!("cancelled {agent_id}"),
            bytes_transferred: None,
            duration: start.elapsed(),
            status: ToolStatus::Success,
        })
    }
}

#[async_trait]
impl Tool for CancelSubagentTool {
    fn name(&self) -> &str {
        "cancel_subagent"
    }

    fn description(&self) -> &str {
        "Cancel a running subagent."
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "agent_id": { "type": "string", "description": "Subagent identifier" }
            },
            "required": ["agent_id"]
        })
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn requires_approval(&self, _input: &Value) -> bool {
        false
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolOutput> {
        self.execute(input, ctx)
    }
}
