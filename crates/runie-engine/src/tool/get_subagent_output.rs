//! `get_subagent_output` orchestrator tool.

use std::time::Instant;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::Value;

use crate::tool::{Tool, ToolContext, ToolOutput, ToolStatus};

/// Gets partial or final output from a subagent.
#[derive(Debug, Clone, Copy, Default)]
pub struct GetSubagentOutputTool;

impl GetSubagentOutputTool {
    pub fn execute(&self, input: Value, ctx: &ToolContext) -> Result<ToolOutput> {
        let start = Instant::now();
        let agent_id = input["agent_id"]
            .as_str()
            .ok_or_else(|| anyhow!("get_subagent_output: missing agent_id"))?;
        let registry = ctx
            .agent_registry
            .as_ref()
            .ok_or_else(|| anyhow!("get_subagent_output: no agent registry in context"))?;
        let guard = registry.lock().map_err(|e| anyhow!("lock poisoned: {e}"))?;
        let output = guard
            .output(agent_id)
            .ok_or_else(|| anyhow!("agent {} has no output yet", agent_id))?;
        Ok(ToolOutput {
            tool_name: "get_subagent_output".to_string(),
            tool_args: input,
            content: output.to_string(),
            bytes_transferred: None,
            duration: start.elapsed(),
            status: ToolStatus::Success,
        })
    }
}

#[async_trait]
impl Tool for GetSubagentOutputTool {
    fn name(&self) -> &str {
        "get_subagent_output"
    }

    fn description(&self) -> &str {
        "Get the partial or final output from a subagent."
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
        true
    }

    fn requires_approval(&self, _input: &Value) -> bool {
        false
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolOutput> {
        self.execute(input, ctx)
    }
}
