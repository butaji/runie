//! Provider trait and message types

use crate::llm_event::LLMEvent;
use anyhow::Result;
use futures::Stream;
use std::pin::Pin;

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

/// A chunk of streaming response (legacy type, prefer LLMEvent).
#[derive(Debug, Clone)]
pub struct ResponseChunk {
    pub content: String,
}

impl From<ResponseChunk> for LLMEvent {
    fn from(chunk: ResponseChunk) -> Self {
        LLMEvent::TextDelta(chunk.content)
    }
}

/// Error constructing or operating a provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderError {
    /// The provider key was not found in the registry.
    UnknownProvider(String),
    /// The API key is missing or invalid for this provider.
    MissingApiKey(String),
    /// Some other error during construction or API call.
    Other(String),
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderError::UnknownProvider(k) => write!(f, "Unknown provider: {}", k),
            ProviderError::MissingApiKey(k) => write!(f, "Missing API key for: {}", k),
            ProviderError::Other(s) => write!(f, "Provider error: {}", s),
        }
    }
}

impl std::error::Error for ProviderError {}

/// Provider trait — implemented by LLM backends.
/// Returns a `Stream` of `LLMEvent`s.
///
/// This trait is dyn-compatible (no `async fn`, no generic parameters).
pub trait Provider: Send + Sync {
    /// Generate a streaming response, returning a stream of LLM events.
    fn generate(
        &self,
        messages: Vec<Message>,
    ) -> Pin<Box<dyn Stream<Item = Result<LLMEvent>> + Send + '_>>;
}
