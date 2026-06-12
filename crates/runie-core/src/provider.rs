//! Provider trait and message types

use anyhow::Result;

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

/// Provider trait — implemented by LLM backends.
/// Streams chunks via the `on_chunk` callback as they arrive from the API.
pub trait Provider: Send + Sync {
    #[allow(async_fn_in_trait)]
    async fn generate<F>(&self, messages: Vec<Message>, on_chunk: F) -> Result<()>
    where
        F: FnMut(ResponseChunk) + Send;
}
