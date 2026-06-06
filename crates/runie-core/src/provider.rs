//! Provider trait and message types

/// Message roles for LLM conversations
#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    System { content: String },
    User { content: String },
    Assistant { content: String },
    ToolResult { content: String },
}

impl Message {
    pub fn role(&self) -> &'static str {
        match self {
            Message::System { .. } => "system",
            Message::User { .. } => "user",
            Message::Assistant { .. } => "assistant",
            Message::ToolResult { .. } => "tool",
        }
    }

    pub fn content(&self) -> &str {
        match self {
            Message::System { content }
            | Message::User { content }
            | Message::Assistant { content }
            | Message::ToolResult { content } => content,
        }
    }
}

/// A chunk of streaming response
#[derive(Debug, Clone)]
pub struct ResponseChunk {
    pub content: String,
}

/// Provider trait — implemented by LLM backends
pub trait Provider: Send {
    fn generate(&self, messages: Vec<Message>) -> Vec<ResponseChunk>;
}
