//! Chat implementation for ReplyProvider.

use async_stream::stream;
use async_trait::async_trait;
use futures::stream::BoxStream;
use runie_core::{Event, Message, ProviderError, ToolSchema};

use super::{ReplyProvider, Scenario};
use super::helpers::extract_content_from_events;

#[async_trait]
impl super::Provider for ReplyProvider {
    fn name(&self) -> &str {
        "reply"
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn supports_tools(&self) -> bool {
        true
    }

    fn supports_vision(&self) -> bool {
        false
    }

    fn max_context_tokens(&self) -> usize {
        128_000
    }

    async fn chat(
        &self,
        messages: Vec<Message>,
        _tools: Vec<ToolSchema>,
    ) -> Result<BoxStream<'static, Event>, ProviderError> {
        let scenario = self.select_scenario(&messages);
        let events = self.generate_events_for_scenario(scenario);

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
        let scenario = self.select_scenario(&messages);
        let events = self.generate_events_for_scenario(scenario);
        Ok(extract_content_from_events(&events))
    }
}

impl ReplyProvider {
    /// Generate events for a given scenario.
    pub fn generate_events_for_scenario(&self, scenario: Scenario) -> Vec<Event> {
        match scenario {
            Scenario::Simple => self.generate_simple_events(),
            Scenario::Tool => self.generate_tool_events(),
            Scenario::Stream => self.generate_stream_events(),
            Scenario::StreamTool => self.generate_stream_tool_events(),
            Scenario::Error => self.generate_error_events(),
            Scenario::Context => self.generate_context_events(),
            Scenario::LongReasoning => self.generate_long_reasoning_events(),
        }
    }

    /// Select scenario based on last user message.
    pub fn select_scenario(&self, messages: &[Message]) -> Scenario {
        for msg in messages.iter().rev() {
            if let Message::User { content, .. } = msg {
                return Scenario::from_input(content);
            }
        }
        Scenario::Simple
    }
}
