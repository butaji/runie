//! `done` tool — explicit subagent completion signal.

use std::time::Instant;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::Value;

use crate::tool::{Tool, ToolContext, ToolOutput, ToolStatus};

/// Built-in tool that signals a subagent has finished its task.
#[derive(Debug, Clone, Copy, Default)]
pub struct DoneTool;

impl DoneTool {
    /// Synchronous execute used in tests.
    pub fn execute(&self, input: Value, ctx: &ToolContext) -> Result<ToolOutput> {
        let start = Instant::now();
        let result = input
            .get("result")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        signal_completion(ctx, &result)?;
        Ok(ToolOutput {
            tool_name: "done".to_string(),
            tool_args: input,
            content: result,
            bytes_transferred: None,
            duration: start.elapsed(),
            status: ToolStatus::Success,
        })
    }
}

fn signal_completion(ctx: &ToolContext, result: &str) -> Result<()> {
    let agent_id = ctx
        .agent_id
        .as_ref()
        .ok_or_else(|| anyhow!("done: no agent_id in context"))?;
    let registry = ctx
        .agent_registry
        .as_ref()
        .ok_or_else(|| anyhow!("done: no agent registry in context"))?;
    let mut guard = registry.lock().map_err(|e| anyhow!("lock poisoned: {e}"))?;
    guard
        .signal_done(agent_id, Some(result.to_string()))
        .map_err(|e| anyhow!("{e}"))?;
    Ok(())
}

#[async_trait]
impl Tool for DoneTool {
    fn name(&self) -> &str {
        "done"
    }

    fn description(&self) -> &str {
        "Signal that the subagent has completed its task and return a result."
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "result": {
                    "type": "string",
                    "description": "Final result to return to the orchestrator."
                }
            },
            "required": ["result"]
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

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use runie_core::multi_agent::AgentRegistry;

    #[test]
    fn done_tool_signals_completion() {
        let registry = Arc::new(Mutex::new(AgentRegistry::default()));
        let id = registry
            .lock()
            .unwrap()
            .spawn("reviewer", "task".to_string(), runie_core::orchestrator::ModelTrait::General, 4000)
            .unwrap();
        let ctx = ToolContext {
            agent_id: Some(id.clone()),
            agent_registry: Some(registry.clone()),
            ..ToolContext::default()
        };
        let tool = DoneTool;
        tool.execute(serde_json::json!({"result": "looks good"}), &ctx)
            .unwrap();
        let guard = registry.lock().unwrap();
        let status = guard.wait(&id).unwrap();
        assert!(matches!(
            status,
            runie_core::orchestrator::AgentLifecycleStatus::Done { .. }
        ));
        assert_eq!(guard.output(&id), Some("looks good"));
    }

    #[test]
    fn done_tool_requires_agent_id() {
        let ctx = ToolContext::default();
        let tool = DoneTool;
        assert!(tool
            .execute(serde_json::json!({"result": "x"}), &ctx)
            .is_err());
    }
}
