use crate::events::*;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct AgentState {
    pub system_prompt: String,
    pub model: String,
    pub thinking_level: String,
    pub tools: Vec<AgentTool>,
    pub messages: Vec<AgentMessage>,
    pub is_streaming: bool,
    pub streaming_message: Option<AgentMessage>,
    pub pending_tool_calls: HashSet<String>,
    pub error_message: Option<String>,
}

impl Default for AgentState {
    fn default() -> Self {
        Self {
            system_prompt: String::new(),
            model: "unknown".to_string(),
            thinking_level: "off".to_string(),
            tools: Vec::new(),
            messages: Vec::new(),
            is_streaming: false,
            streaming_message: None,
            pending_tool_calls: HashSet::new(),
            error_message: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AgentTool {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}
