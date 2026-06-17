//! `list_subagents` orchestrator tool.

use std::time::Instant;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::Value;

use crate::tool::{Tool, ToolContext, ToolOutput, ToolStatus};

/// Lists all active subagents with their status.
#[derive(Debug, Clone, Copy, Default)]
pub struct ListSubagentsTool;

impl ListSubagentsTool {
    pub fn execute(&self, input: Value, ctx: &ToolContext) -> Result<ToolOutput> {
        let start = Instant::now();
        let registry = ctx
            .agent_registry
            .as_ref()
            .ok_or_else(|| anyhow!("list_subagents: no agent registry in context"))?;
        let guard = registry.lock().map_err(|e| anyhow!("lock poisoned: {e}"))?;
        let list: Vec<Value> = guard
            .list()
            .into_iter()
            .map(|entry| {
                serde_json::json!({
                    "id": entry.agent_id,
                    "role": entry.role,
                    "status": entry.status.label()
                })
            })
            .collect();
        Ok(ToolOutput {
            tool_name: "list_subagents".to_string(),
            tool_args: input,
            content: serde_json::to_string_pretty(&list)?,
            bytes_transferred: None,
            duration: start.elapsed(),
            status: ToolStatus::Success,
        })
    }
}

#[async_trait]
impl Tool for ListSubagentsTool {
    fn name(&self) -> &str {
        "list_subagents"
    }

    fn description(&self) -> &str {
        "List all active subagents with their current status."
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
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
