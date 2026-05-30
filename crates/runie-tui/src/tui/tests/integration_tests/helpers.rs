use runie_agent::{AgentEvent, AgentMessage, ContentPart, ToolResult, TokenUsage};

/// Create an AgentMessage with the given role and text content
pub fn agent_message(role: &str, text: &str) -> AgentMessage {
    AgentMessage {
        role: role.to_string(),
        content: vec![ContentPart::Text {
            text: text.to_string(),
        }],
        timestamp: 0,
        usage: None,
        stop_reason: None,
        error_message: None,
        tool_calls: vec![],
    }
}

/// Create a default TokenUsage
pub fn default_token_usage() -> TokenUsage {
    TokenUsage {
        input: 100,
        output: 50,
        cache_read: 0,
        cache_write: 0,
        total_tokens: 150,
    }
}
