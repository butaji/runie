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

    #[must_use]
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

        if let Some((tool_name, tool_args)) = Self::should_use_tools(&content, tools) {
            Self::build_tool_response(&tool_name, &tool_args)
        } else {
            Self::build_text_response(&content)
        }
    }

    fn build_tool_response(name: &str, args: &str) -> Vec<Event> {
        let duration_ms = 150 + (rand::random::<u64>() % 300);
        vec![
            Event::AgentStart { session_id: "mock-session".to_string(), timestamp: Utc::now() },
            Event::MessageStart { role: "assistant".to_string(), timestamp: Utc::now() },
            Event::MessageDelta { content: format!("Let me {} for you...\n\n", name.to_lowercase()) },
            Event::ToolCallDelta {
                id: "mock-1".to_string(),
                name: name.to_string(),
                arguments: args.to_string(),
            },
            Event::ToolExecutionStart {
                tool_call_id: "mock-1".to_string(),
                tool_name: name.to_string(),
                args: serde_json::Value::String(args.to_string()),
                timestamp: Utc::now(),
            },
            Event::ToolExecutionEnd {
                tool_call_id: "mock-1".to_string(),
                result: ToolOutput {
                    content: format!("{} completed successfully", name),
                    metadata: serde_json::json!({ "duration_ms": duration_ms }),
                    terminate: false,
                },
                timestamp: Utc::now(),
            },
            Event::MessageEnd,
            Event::AgentEnd { timestamp: Utc::now() },
        ]
    }

    fn build_text_response(content: &str) -> Vec<Event> {
        let thinking = Self::thinking_for_text(content);
        vec![
            Event::AgentStart { session_id: "mock-session".to_string(), timestamp: Utc::now() },
            Event::MessageStart { role: "assistant".to_string(), timestamp: Utc::now() },
            Event::MessageDelta { content: thinking },
            Event::MessageDelta { content: content.to_string() },
            Event::MessageEnd,
            Event::AgentEnd { timestamp: Utc::now() },
        ]
    }

    fn thinking_for_text(text: &str) -> String {
        let lower = text.to_lowercase();
        if lower.contains("hello") || lower.contains("hi") {
            return "The user said \"hello\". They want a friendly greeting.\n\n".to_string();
        }
        if lower.contains("list") {
            return "The user mentioned listing files. I'll show them what files are available.\n\n".to_string();
        }
        if lower.contains("read") {
            return "The user wants to read something. I'll retrieve the content for them.\n\n".to_string();
        }
        if lower.contains("edit") || lower.contains("fix") {
            return "The user wants to edit a file. I should first understand the current content.\n\n".to_string();
        }
        let preview: String = text.chars().take(30).collect();
        format!("The user said \"{}\". This is a request I need to handle.\n\n", preview)
    }

    fn should_use_tools(content: &str, tools: &[ToolSchema]) -> Option<(String, String)> {
        if tools.is_empty() {
            return None;
        }
        let lower = content.to_lowercase();
        let mentions_tool = lower.contains("edit") || lower.contains("list") || lower.contains("read");
        if !mentions_tool {
            return None;
        }
        let (tool_name, tool_args) = Self::detect_tool_and_args(&lower, tools);
        Some((tool_name, tool_args))
    }

    fn detect_tool_and_args(lower: &str, tools: &[ToolSchema]) -> (String, String) {
        let name = if lower.contains("read") {
            "Read".to_string()
        } else if lower.contains("list") {
            "List".to_string()
        } else {
            tools[0].name.clone()
        };
        let args = if lower.contains("list") { ".".to_string() } else { "{}".to_string() };
        (name, args)
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
            return "Hello! 👋 How can I help you today?".to_string();
        }
        if lower.contains("list") {
            return "I can help you list files. 📁 What directory would you like to see?".to_string();
        }
        if lower.contains("read") {
            return "I'll read that for you. 📖 Which file are you interested in?".to_string();
        }
        if lower.contains("edit") || lower.contains("fix") {
            return "I can help with that edit. ✏️ Let me take a look at the current content first.".to_string();
        }
        if lower.contains("test") {
            return "I'll run those tests for you. 🧪 Let me check your test setup.".to_string();
        }
        let preview: String = text.chars().take(50).collect();
        format!("I see: \"{}\". 🔧 How can I assist you with this?", preview)
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

        let delay = self.response_delay_ms;
        let s = stream! {
            for event in events {
                yield event;
                if delay > 0 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                }
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
