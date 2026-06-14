use crate::model::ThinkingLevel;
use crate::provider::Message;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StopReason {
    Stop,
    Length,
    ToolUse,
    ContentFilter,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone)]
pub enum LLMEvent {
    TextDelta(String),
    ThinkingDelta(String),
    ToolCallStart {
        id: String,
        name: String,
    },
    ToolCallInputDelta {
        id: String,
        delta: String,
    },
    ToolCallEnd {
        id: String,
    },
    Error(String),
    Usage {
        input_tokens: usize,
        output_tokens: usize,
    },
    Finish {
        reason: StopReason,
    },
}

#[derive(Debug, Clone)]
pub struct LLMRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub tools: Vec<ToolSchema>,
    pub system_prompt: Option<String>,
    pub max_tokens: Option<usize>,
    pub thinking_level: ThinkingLevel,
}
