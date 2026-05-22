use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AgentEvent {
    MessageStart { message: AgentMessage },
    MessageUpdate { message: AgentMessage },
    MessageEnd { message: AgentMessage },
    ToolExecutionStart { tool_call_id: String },
    ToolExecutionEnd { tool_call_id: String, result: ToolResult },
    TurnEnd { message: AgentMessage, tool_results: Vec<ToolResult> },
    AgentEnd { messages: Vec<AgentMessage> },
    TokenUsage { prompt_tokens: usize, completion_tokens: usize, total_tokens: usize },
    Error { message: String },
    PermissionRequest { tool_call_id: String, tool_name: String, tool_args: String },
    PermissionGranted { tool_call_id: String },
    PermissionDenied { tool_call_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionDecision {
    Allow { tool_call_id: String },
    Deny { tool_call_id: String },
    AllowAlways { tool_call_id: String },
    Skip { tool_call_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub role: String, // "user", "assistant", "tool", "system"
    pub content: Vec<ContentPart>,
    pub timestamp: i64,
    pub usage: Option<TokenUsage>,
    pub stop_reason: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentPart {
    Text { text: String },
    Image { source: ImageSource },
    ToolUse { id: String, name: String, input: serde_json::Value },
    ToolResult { tool_use_id: String, content: Vec<ContentPart>, is_error: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSource {
    pub media_type: String,
    pub data: String, // base64
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input: u32,
    pub output: u32,
    pub cache_read: u32,
    pub cache_write: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub tool_name: String,
    pub input: serde_json::Value,
    pub content: Vec<ContentPart>,
    pub is_error: bool,
}
