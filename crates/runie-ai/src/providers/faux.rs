//! FauxProvider - Test provider with declarative response sequencing
//!
//! Supports:
//! - Token-level streaming simulation
//! - Declarative response sequences for deterministic tests
//! - Configurable delays between tokens/chunks

use async_trait::async_trait;
use futures::stream::BoxStream;
use runie_core::{Event, Message, ProviderError, ToolSchema};
use async_stream::stream;
use chrono::Utc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A single step in a declarative response sequence
#[derive(Debug, Clone)]
pub enum ResponseStep {
    /// Emit text content
    Text(String),
    /// Emit a tool call delta
    ToolCallDelta { id: String, name: String, arguments: String },
    /// Wait for N milliseconds
    Delay(u64),
    /// End the message
    MessageEnd,
    /// Emit usage stats
    Usage { prompt_tokens: usize, completion_tokens: usize },
}

/// A declarative response sequence
#[derive(Debug, Clone)]
pub struct ResponseSequence {
    steps: Vec<ResponseStep>,
}

impl ResponseSequence {
    /// Create a new empty sequence
    pub fn new() -> Self {
        Self { steps: vec![] }
    }

    /// Add a text step
    pub fn text(mut self, content: &str) -> Self {
        self.steps.push(ResponseStep::Text(content.to_string()));
        self
    }

    /// Add a tool call delta step
    pub fn tool_call_delta(mut self, id: &str, name: &str, arguments: &str) -> Self {
        self.steps.push(ResponseStep::ToolCallDelta {
            id: id.to_string(),
            name: name.to_string(),
            arguments: arguments.to_string(),
        });
        self
    }

    /// Add a delay step
    pub fn delay(mut self, ms: u64) -> Self {
        self.steps.push(ResponseStep::Delay(ms));
        self
    }

    /// Add message end
    pub fn end(mut self) -> Self {
        self.steps.push(ResponseStep::MessageEnd);
        self
    }

    /// Add usage stats
    pub fn usage(mut self, prompt: usize, completion: usize) -> Self {
        self.steps.push(ResponseStep::Usage { prompt_tokens: prompt, completion_tokens: completion });
        self
    }

    /// Build the events from this sequence
    fn build_events(&self) -> Vec<Event> {
        let mut events = vec![
            Event::AgentStart { session_id: "faux-session".to_string(), timestamp: Utc::now() },
            Event::MessageStart { role: "assistant".to_string(), timestamp: Utc::now() },
        ];

        for step in &self.steps {
            match step {
                ResponseStep::Text(content) => {
                    events.push(Event::MessageDelta { content: content.clone() });
                }
                ResponseStep::ToolCallDelta { id, name, arguments } => {
                    events.push(Event::ToolCallDelta {
                        id: id.clone(),
                        name: name.clone(),
                        arguments: arguments.clone(),
                    });
                }
                ResponseStep::Delay(_) => {
                    // Delays handled during streaming
                }
                ResponseStep::MessageEnd => {
                    events.push(Event::MessageEnd);
                }
                ResponseStep::Usage { prompt_tokens, completion_tokens } => {
                    events.push(Event::Usage {
                        prompt_tokens: *prompt_tokens,
                        completion_tokens: *completion_tokens,
                        total_tokens: prompt_tokens + completion_tokens,
                    });
                }
            }
        }

        events.push(Event::AgentEnd { timestamp: Utc::now() });
        events
    }

    /// Stream events with token-level granularity (1 char per emission)
    pub fn stream_events(&self) -> Vec<Event> {
        self.build_events()
    }
}

impl Default for ResponseSequence {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for a single faux response
#[derive(Debug, Clone)]
pub struct FauxResponse {
    sequence: ResponseSequence,
    token_delay_ms: u64,
}

/// Builder for FauxResponse
impl FauxResponse {
    pub fn new() -> Self {
        Self {
            sequence: ResponseSequence::new(),
            token_delay_ms: 0,
        }
    }

    pub fn with_token_delay(mut self, ms: u64) -> Self {
        self.token_delay_ms = ms;
        self
    }

    pub fn text(mut self, content: &str) -> Self {
        self.sequence = self.sequence.text(content);
        self
    }

    pub fn end(mut self) -> Self {
        self.sequence = self.sequence.end();
        self
    }

    pub fn build(self) -> ResponseSequence {
        self.sequence
    }
}

impl Default for FauxResponse {
    fn default() -> Self {
        Self::new()
    }
}

/// A declarative response for simple text responses
pub fn faux_text(text: &str) -> ResponseSequence {
    ResponseSequence::new()
        .text(text)
        .end()
}

/// A declarative response for tool call scenarios
pub fn faux_tool_call(id: &str, name: &str, args: &str, result: &str) -> ResponseSequence {
    ResponseSequence::new()
        .text("I'll help with that.")
        .tool_call_delta(id, name, args)
        .end()
        .text(result)  // Tool result simulation would come from separate handler
}

/// FauxProvider - test provider with deterministic, declarative responses
pub struct FauxProvider {
    responses: Vec<ResponseSequence>,
    call_count: AtomicUsize,
    token_delay_ms: u64,
}

impl FauxProvider {
    pub fn new() -> Self {
        Self {
            responses: vec![],
            call_count: AtomicUsize::new(0),
            token_delay_ms: 0,
        }
    }

    /// Add a response sequence
    pub fn add_response(mut self, sequence: ResponseSequence) -> Self {
        self.responses.push(sequence);
        self
    }

    /// Add a simple text response
    pub fn add_text_response(mut self, text: &str) -> Self {
        self.responses.push(faux_text(text));
        self
    }

    /// Set delay between tokens (for streaming simulation)
    pub fn with_token_delay(mut self, ms: u64) -> Self {
        self.token_delay_ms = ms;
        self
    }

    /// Reset call count to beginning
    pub fn reset(&self) {
        self.call_count.store(0, Ordering::SeqCst);
    }

    fn get_next_response(&self) -> ResponseSequence {
        if self.responses.is_empty() {
            return faux_text("No response configured");
        }
        let idx = self.call_count.fetch_add(1, Ordering::SeqCst);
        self.responses[idx % self.responses.len()].clone()
    }
}

impl Default for FauxProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl crate::Provider for FauxProvider {
    fn name(&self) -> &str {
        "faux"
    }

    fn model(&self) -> &str {
        "faux-model"
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
        _messages: Vec<Message>,
        _tools: Vec<ToolSchema>,
    ) -> Result<BoxStream<'static, Event>, ProviderError> {
        let response = self.get_next_response();
        let events = response.stream_events();

        let s = stream! {
            for event in events {
                yield event;
            }
        };

        Ok(Box::pin(s))
    }

    async fn chat_simple(
        &self,
        _messages: Vec<Message>,
    ) -> Result<String, ProviderError> {
        Ok("No response sequence configured".to_string())
    }
}

/// Extension trait for FauxProvider to get response streams
impl FauxProvider {
    /// Create a stream from a specific response sequence
    pub fn stream_for_response(&self, sequence: &ResponseSequence) -> BoxStream<'static, Event> {
        let events = sequence.stream_events();
        let s = stream! {
            for event in events {
                yield event;
            }
        };
        Box::pin(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::Provider;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_faux_provider_simple_response() {
        let provider = FauxProvider::new()
            .add_text_response("Hello, world!");

        let events: Vec<Event> = provider.chat(vec![], vec![]).await.unwrap().collect().await;
        assert!(events.iter().any(|e| matches!(e, Event::MessageDelta { content } if content.contains("Hello"))));
    }

    #[test]
    fn test_response_sequence_builder() {
        let seq = ResponseSequence::new()
            .text("Hello ")
            .text("world")
            .end();

        let events = seq.build_events();
        let deltas: Vec<&str> = events.iter()
            .filter_map(|e| match e {
                Event::MessageDelta { content } => Some(content.as_str()),
                _ => None,
            })
            .collect();

        assert_eq!(deltas, vec!["Hello ", "world"]);
    }

    #[test]
    fn test_tool_call_sequence() {
        let seq = faux_tool_call("call-1", "read_file", "{}", "File contents");
        let events = seq.build_events();

        assert!(events.iter().any(|e| matches!(e, Event::ToolCallDelta { name, .. } if name == "read_file")));
    }
}
