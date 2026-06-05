use crate::types::{Message, ToolCall};
use async_trait::async_trait;

#[async_trait]
pub trait Provider: Send + Sync {
    async fn chat(
        &self,
        messages: Vec<Message>,
    ) -> Result<(String, Vec<ToolCall>), ProviderError>;
}

#[derive(Debug, thiserror::Error, Clone)]
pub enum ProviderError {
    #[error("provider error: {0}")]
    Other(String),
}

pub struct MockProvider;

#[async_trait]
impl Provider for MockProvider {
    async fn chat(
        &self,
        messages: Vec<Message>,
    ) -> Result<(String, Vec<ToolCall>), ProviderError> {
        let last = messages.iter().rev().find_map(|m| match m {
            Message::User { content, .. } => Some(content.as_str()),
            _ => None,
        });

        let reply = match last {
            Some(text) if text.to_lowercase().contains("hello") => {
                "Hello! I'm a mock agent. How can I help you today?".into()
            }
            Some(text) => format!("Echo: {}", text),
            None => "Ready.".into(),
        };

        Ok((reply, vec![]))
    }
}
