//! Typed subagent requests. Execution is supplied by the owning loop actor.

use crate::types::{AgentTool, AgentToolResult, ToolResultContent};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubagentRole {
    Explore,
    Plan,
    Code,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SubagentRequest {
    pub role: SubagentRole,
    pub task: String,
    #[serde(default)]
    pub context: String,
}

#[derive(Default)]
pub struct SubagentTool;

#[async_trait::async_trait]
impl AgentTool for SubagentTool {
    fn name(&self) -> &str {
        "subagent"
    }
    fn label(&self) -> &str {
        "Run subagent"
    }
    fn description(&self) -> &str {
        "Run an isolated explore, plan, or code task."
    }
    fn parameters(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "type": "object",
            "properties": {
                "role": { "type": "string", "enum": ["explore", "plan", "code"] },
                "task": { "type": "string", "minLength": 1 },
                "context": { "type": "string" }
            },
            "required": ["role", "task"]
        }))
    }
    fn validate_arguments(&self, args: &serde_json::Value) -> Result<(), String> {
        let request: SubagentRequest = serde_json::from_value(args.clone())
            .map_err(|error| format!("invalid subagent request: {error}"))?;
        if request.task.trim().is_empty() {
            return Err("subagent task must not be empty".into());
        }
        if request.task.chars().count() > 20_000 {
            return Err("subagent task is too long".into());
        }
        Ok(())
    }
    async fn execute(
        &self,
        _tool_call_id: &str,
        _args: serde_json::Value,
        _signal: Option<tokio_util::sync::CancellationToken>,
        _on_update: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        Err("subagent requires an owning subagent hook".into())
    }
}

pub(crate) fn result(value: serde_json::Value) -> AgentToolResult {
    AgentToolResult {
        content: vec![ToolResultContent::Text {
            text: value.to_string(),
        }],
        details: value,
        ..AgentToolResult::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_role_and_non_empty_task() {
        let tool = SubagentTool;
        assert!(tool
            .validate_arguments(&serde_json::json!({
                "role": "explore", "task": "find the provider boundary"
            }))
            .is_ok());
        assert!(tool
            .validate_arguments(&serde_json::json!({
                "role": "code", "task": " "
            }))
            .is_err());
        assert!(tool
            .validate_arguments(&serde_json::json!({
                "role": "unknown", "task": "inspect"
            }))
            .is_err());
    }
}
