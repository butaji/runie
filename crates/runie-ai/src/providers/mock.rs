use async_trait::async_trait;
use runie_core::{Message, ToolSchema, Event, ProviderError, ToolOutput};
use futures::stream::BoxStream;
use crate::Provider;
use async_stream::stream;
use chrono::Utc;

/// A mock provider for development/testing that simulates LLM responses.
pub struct MockProvider {
    model: String,
    response_delay_ms: u64,
    simulate_errors: bool,
    error_rate: f32,
    simulate_rate_limit: bool,
}

impl MockProvider {
    pub fn new() -> Self {
        Self {
            model: "mock-gpt-4".to_string(),
            response_delay_ms: 100,
            simulate_errors: false,
            error_rate: 0.0,
            simulate_rate_limit: false,
        }
    }

    pub fn with_delay(mut self, ms: u64) -> Self {
        self.response_delay_ms = ms;
        self
    }

    pub fn with_errors(mut self, rate: f32) -> Self {
        self.simulate_errors = true;
        self.error_rate = rate.clamp(0.0, 1.0);
        self
    }

    pub fn with_rate_limit_simulation(mut self) -> Self {
        self.simulate_rate_limit = true;
        self
    }

    fn generate_response(&self, messages: &[Message], tools: &[ToolSchema]) -> Vec<Event> {
        let content = Self::build_content(messages);

        // If user asks about editing and tools are available, simulate tool call
        if !tools.is_empty() && content.to_lowercase().contains("edit") {
            let tool = &tools[0];
            vec![
                Event::AgentStart { session_id: "mock-session".to_string(), timestamp: Utc::now() },
                Event::MessageStart { role: "assistant".to_string(), timestamp: Utc::now() },
                Event::MessageDelta { content: "I'll help you with that.".to_string() },
                Event::ToolCallDelta {
                    name: tool.name.clone(),
                    arguments: "{}".to_string()
                },
                Event::MessageEnd,
                Event::ToolExecutionStart {
                    tool_call_id: "mock-1".to_string(),
                    tool_name: tool.name.clone(),
                    args: serde_json::json!({}),
                    timestamp: Utc::now(),
                },
                Event::ToolExecutionEnd {
                    tool_call_id: "mock-1".to_string(),
                    result: ToolOutput { content: "Done".to_string(), metadata: serde_json::json!({}), terminate: false },
                    timestamp: Utc::now(),
                },
                Event::AgentEnd { timestamp: Utc::now() },
            ]
        } else {
            // Normal text response
            vec![
                Event::AgentStart { session_id: "mock-session".to_string(), timestamp: Utc::now() },
                Event::MessageStart { role: "assistant".to_string(), timestamp: Utc::now() },
                Event::MessageDelta { content },
                Event::MessageEnd,
                Event::AgentEnd { timestamp: Utc::now() },
            ]
        }
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
        let lower = text.to_lowercase();
        if lower.split_whitespace().any(|w| w == "hello" || w == "hi") {
            return "Hello! I'm a mock coding agent. How can I help you today?".to_string();
        }
        if lower.contains("edit") || lower.contains("fix") {
            return "I'll help you edit that file. Let me first read it to understand the current state.".to_string();
        }
        if lower.contains("test") {
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

    fn supports_tools(&self) -> bool {
        true
    }

    fn supports_vision(&self) -> bool {
        true
    }

    fn max_context_tokens(&self) -> usize {
        128_000
    }

    async fn chat(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolSchema>,
    ) -> Result<BoxStream<'static, Event>, ProviderError> {
        if self.simulate_errors && rand::random::<f32>() < self.error_rate {
            return Err(ProviderError::ApiError("Simulated error".to_string()));
        }
        if self.simulate_rate_limit && rand::random::<f32>() < 0.3 {
            return Err(ProviderError::RateLimited);
        }

        let events = self.generate_response(&messages, &tools);

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
        let events = self.generate_response(&messages, &[]);
        let mut content = String::new();

        for event in events {
            if let Event::MessageDelta { content: c } = event {
                content.push_str(&c);
            }
        }

        Ok(content)
    }
}
