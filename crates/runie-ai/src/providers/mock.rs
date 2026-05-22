use async_trait::async_trait;
use runie_core::{Message, ToolSchema, Event, ProviderError};
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
        let content = Self::build_content(messages);
        vec![
            Event::AgentStart { session_id: "mock-session".to_string(), timestamp: Utc::now() },
            Event::MessageStart { role: "assistant".to_string(), timestamp: Utc::now() },
            Event::MessageDelta { content: content.clone() },
            Event::MessageEnd,
            Event::AgentEnd { timestamp: Utc::now() },
        ]
    }

    fn build_content(messages: &[Message]) -> String {
        let last = messages.iter().rev().find(|m| matches!(m, Message::User { .. }));
        let text = match last {
            Some(Message::User { content, .. }) => content.as_str(),
            _ => return Self::default_response(),
        };
        Self::response_for_text(text)
    }

    fn default_response() -> String {
        "I'm ready to help! What would you like to work on?".to_string()
    }

    fn response_for_text(text: &str) -> String {
        if text.contains("hello") || text.contains("hi") {
            return "Hello! I'm a mock coding agent. How can I help you today?".to_string();
        }
        if text.contains("edit") || text.contains("fix") {
            return "I'll help you edit that file. Let me first read it to understand the current state.".to_string();
        }
        if text.contains("test") {
            return "I'll run the tests for you. Let me check what test framework you're using.".to_string();
        }
        let preview = &text[..text.len().min(50)];
        format!("I received your message: \"{}\". This is a mock response for testing.", preview)
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

        let s = stream! {
            for event in events {
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