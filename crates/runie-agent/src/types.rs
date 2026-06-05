use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Message {
    System { content: String },
    User { content: String },
    Assistant { content: String, tool_calls: Vec<ToolCall> },
    ToolResult { tool_call_id: String, content: String, is_error: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AgentEvent {
    MessageStart { role: String },
    MessageDelta { content: String },
    MessageEnd,
    ToolCallStart { id: String, name: String },
    ToolCallEnd { id: String, result: String },
    Error { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolOutput {
    pub content: String,
}

impl Default for ToolOutput {
    fn default() -> Self {
        Self { content: String::new() }
    }
}

#[derive(Debug, thiserror::Error, Clone, PartialEq)]
pub enum ToolError {
    #[error("execution failed: {0}")]
    ExecutionFailed(String),
}
