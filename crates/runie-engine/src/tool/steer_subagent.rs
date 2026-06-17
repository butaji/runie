//! `steer_subagent` orchestrator tool.

use std::time::Instant;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::Value;

use crate::tool::{Tool, ToolContext, ToolOutput, ToolStatus};

/// Sends a message to a running subagent.
#[derive(Debug, Clone, Copy, Default)]
pub struct SteerSubagentTool;

impl SteerSubagentTool {
    pub fn execute(&self, input: Value, ctx: &ToolContext) -> Result<ToolOutput> {
        let start = Instant::now();
        let agent_id = input["agent_id"]
            .as_str()
            .ok_or_else(|| anyhow!("steer_subagent: missing agent_id"))?
            .to_string();
        let message = input["message"]
            .as_str()
            .ok_or_else(|| anyhow!("steer_subagent: missing message"))?
            .to_string();
        let registry = ctx
            .agent_registry
            .as_ref()
            .ok_or_else(|| anyhow!("steer_subagent: no agent registry in context"))?;
        let mut guard = registry.lock().map_err(|e| anyhow!("lock poisoned: {e}"))?;
        guard.send(agent_id.clone(), message).map_err(|e| anyhow!("{e}"))?;
        Ok(ToolOutput {
            tool_name: "steer_subagent".to_string(),
            tool_args: input,
            content: format!("steered {agent_id}"),
            bytes_transferred: None,
            duration: start.elapsed(),
            status: ToolStatus::Success,
        })
    }
}

#[async_trait]
impl Tool for SteerSubagentTool {
    fn name(&self) -> &str {
        "steer_subagent"
    }

    fn description(&self) -> &str {
        "Send a message to a running subagent."
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "agent_id": { "type": "string", "description": "Subagent identifier" },
                "message": { "type": "string", "description": "Message to send" }
            },
            "required": ["agent_id", "message"]
        })
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn requires_approval(&self, _input: &Value) -> bool {
        false
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolOutput> {
        self.execute(input, ctx)
    }
}
