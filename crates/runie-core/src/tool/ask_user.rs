//! `AskUser` tool — pauses the Orchestrator to ask a clarifying question.
//!
//! When the Orchestrator calls this tool during planning, execution halts and
//! the question is surfaced in the feed. The user's reply is fed back into
//! `OrchestratorContext::record_answer()` so the plan can be refined in one shot.

use std::time::Duration;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::Value;

#[cfg(test)]
use serde_json::json;

use crate::tool::{Tool, ToolContext, ToolOutput, ToolStatus};

/// Built-in tool that asks the user a clarifying question.
///
/// Does not perform any I/O — it signals the runtime to pause planning,
/// display the question, and collect the answer before resuming.
#[derive(Debug, Clone, Copy, Default)]
pub struct AskUserTool;

impl AskUserTool {
    /// Synchronous execute — used in tests. Validates the input and returns
    /// the pending output without performing any I/O.
    pub fn execute(&self, input: Value) -> Result<ToolOutput> {
        let question = input
            .get("question")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("ask_user: missing required field 'question'"))?
            .to_string();

        Ok(ToolOutput {
            tool_name: "ask_user".into(),
            tool_args: input,
            content: question,
            bytes_transferred: None,
            duration: Duration::ZERO,
            status: ToolStatus::AwaitingUser,
        })
    }
}

#[async_trait]
impl Tool for AskUserTool {
    fn name(&self) -> &str {
        "ask_user"
    }

    fn description(&self) -> &str {
        "Ask the user a clarifying question before proceeding. \
         Pauses Orchestrator planning until the user provides an answer."
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "question": {
                    "type": "string",
                    "description": "The question to ask the user."
                }
            },
            "required": ["question"]
        })
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn requires_approval(&self, _input: &Value) -> bool {
        false
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolOutput> {
        // Validate synchronously, then return pending status.
        self.execute(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ask_user_tool_requires_question() {
        let tool = AskUserTool;
        let result = tool.execute(json!({}));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("question"));
    }

    #[test]
    fn ask_user_tool_returns_pending_marker() {
        let tool = AskUserTool;
        let result = tool
            .execute(json!({"question": "Which file?"}))
            .unwrap();
        assert_eq!(result.status, ToolStatus::AwaitingUser);
        assert_eq!(result.content, "Which file?");
        assert_eq!(result.tool_name, "ask_user");
    }

    #[test]
    fn ask_user_tool_null_question() {
        let tool = AskUserTool;
        // null is not a string
        let result = tool.execute(json!({"question": null}));
        assert!(result.is_err());
    }

    #[test]
    fn ask_user_tool_empty_question() {
        let tool = AskUserTool;
        // Empty string is a valid question (user might want to clarify)
        let result = tool.execute(json!({"question": ""}));
        assert!(result.is_ok());
        assert_eq!(result.unwrap().content, "");
    }
}
