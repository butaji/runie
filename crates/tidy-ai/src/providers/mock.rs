use async_trait::async_trait;
use tidy_core::{Message, ToolSchema, Event, ProviderError};
use futures::stream::BoxStream;
use crate::Provider;
use async_stream::stream;
use chrono::Utc;

/// A mock provider for development/testing that simulates LLM responses.
pub struct MockProvider {
    model: String,
    response_delay_ms: u64,
}

impl MockProvider {
    pub fn new() -> Self {
        Self {
            model: "mock-gpt-4".to_string(),
            response_delay_ms: 100,
        }
    }

    pub fn with_delay(mut self, ms: u64) -> Self {
        self.response_delay_ms = ms;
        self
    }

    fn generate_response(&self, messages: &[Message]) -> Vec<Event> {
        let last_message = messages.iter().rev().find(|m| matches!(m, Message::User { .. }));

        let content = match last_message {
            Some(Message::User { content, .. }) => {
                if content.contains("hello") || content.contains("hi") {
                    "Hello! I'm a mock coding agent. How can I help you today?".to_string()
                } else if content.contains("edit") || content.contains("fix") {
                    "I'll help you edit that file. Let me first read it to understand the current state.".to_string()
                } else if content.contains("test") {
                    "I'll run the tests for you. Let me check what test framework you're using.".to_string()
                } else {
                    format!("I received your message: \"{}\". This is a mock response for testing.", &content[..content.len().min(50)])
                }
            }
            _ => "I'm ready to help! What would you like to work on?".to_string(),
        };

        vec![
            Event::AgentStart {
                session_id: "mock-session".to_string(),
                timestamp: Utc::now(),
            },
            Event::MessageStart {
                role: "assistant".to_string(),
                timestamp: Utc::now(),
            },
            Event::MessageDelta {
                content: content.clone(),
            },
            Event::MessageEnd,
            Event::AgentEnd {
                timestamp: Utc::now(),
            },
        ]
    }
}

#[async_trait]
impl Provider for MockProvider {
    fn name(&self) -> &str {
        "mock"
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn chat(
        &self,
        messages: Vec<Message>,
        _tools: Vec<ToolSchema>,
    ) -> Result<BoxStream<'static, Event>, ProviderError> {
        let events = self.generate_response(&messages);
        let delay = tokio::time::Duration::from_millis(self.response_delay_ms);

        let s = stream! {
            for event in events {
                tokio::time::sleep(delay).await;
                yield event;
            }
        };

        Ok(Box::pin(s))
    }

    async fn chat_simple(
        &self,
        messages: Vec<Message>,
    ) -> Result<String, ProviderError> {
        let events = self.generate_response(&messages);
        let mut content = String::new();

        for event in events {
            if let Event::MessageDelta { content: c } = event {
                content.push_str(&c);
            }
        }

        Ok(content)
    }
}