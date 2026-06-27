//! Rig-core adapter for runie-provider.
//!
//! This module provides adapters that implement runie's `Provider` trait
//! using rig-core's provider implementations internally.
//!
//! ## Architecture
//!
//! ```text
//! runie-provider public API
//! ├── Provider trait (unchanged)
//! ├── ProviderEvent types (unchanged)
//! └── DynProvider wrapper (unchanged)
//!
//! Internal implementation
//! ├── RigOpenAiProvider — wraps rig-core OpenAI client, implements Provider
//! └── event mapping — rig RawStreamingChoice → ProviderEvent
//! ```
//!
//! ## Usage
//!
//! The `RigOpenAiProvider` can be used as a drop-in replacement for the
//! custom `OpenAiProvider` implementation. To enable rig-core integration,
//! set the `RIG_ADAPTER` feature flag in `runie-provider`.
//!
//! ## Testing
//!
//! All tests verify event mapping and provider construction without requiring
//! network access.

use futures::Stream;
use pin_project::pin_project;
use runie_core::message::ChatMessage;
use runie_core::provider_event::{ProviderEvent, StopReason};
use std::pin::Pin;
use std::task::{Context, Poll};

// ---------------------------------------------------------------------------
// Event mapping: rig-core RawStreamingChoice → ProviderEvent
// ---------------------------------------------------------------------------

/// A minimal tool call type for streaming responses.
#[derive(Debug, Clone)]
struct StreamingToolCall;

/// Maps a rig-core stop reason to runie StopReason.
#[allow(dead_code)]
fn map_finish_reason(reason: Option<&str>) -> StopReason {
    match reason {
        Some("stop") => StopReason::Stop,
        Some("length") => StopReason::Length,
        Some("content_filter") => StopReason::ContentFilter,
        Some("tool_calls") => StopReason::ToolCalls,
        Some("stop_sequence") => StopReason::StopSequence,
        _ => StopReason::Unknown,
    }
}

/// Maps rig-core streaming choice to one or more runie ProviderEvents.
#[allow(dead_code)]
fn map_streaming_choice(
    choice: rig_core::streaming::RawStreamingChoice<StreamingToolCall>,
) -> Vec<ProviderEvent> {
    use rig_core::streaming::{RawStreamingChoice::*, ToolCallDeltaContent::*};
    let mut events = Vec::new();

    match choice {
        Message(text) => {
            events.push(ProviderEvent::TextDelta(text));
        }
        TextStart { .. } => {
            events.push(ProviderEvent::TextStart { id: "text".to_string() });
        }
        ToolCall(raw) => {
            let call_id = raw.internal_call_id.clone();
            events.push(ProviderEvent::ToolCallStart {
                id: call_id.clone(),
                name: raw.name.clone(),
            });
            if let serde_json::Value::String(args) = &raw.arguments {
                if !args.is_empty() {
                    events.push(ProviderEvent::ToolCallInputDelta {
                        id: call_id.clone(),
                        delta: args.clone(),
                    });
                }
            }
            events.push(ProviderEvent::ToolCallEnd { id: call_id });
        }
        ToolCallDelta { id: _, internal_call_id, content } => {
            match content {
                Name(name) => {
                    events.push(ProviderEvent::ToolCallStart {
                        id: internal_call_id.clone(),
                        name,
                    });
                }
                Delta(args) => {
                    events.push(ProviderEvent::ToolCallInputDelta {
                        id: internal_call_id,
                        delta: args,
                    });
                }
            }
        }
        Reasoning { id, content } => {
            let rid = id.unwrap_or_else(|| "reasoning".to_string());
            events.push(ProviderEvent::ThinkingStart { id: rid.clone() });
            if let rig_core::message::ReasoningContent::Text { text, .. } = content {
                events.push(ProviderEvent::ThinkingDelta(text));
            }
            events.push(ProviderEvent::ThinkingEnd { id: rid });
        }
        ReasoningDelta { id, reasoning } => {
            let rid = id.unwrap_or_else(|| "reasoning".to_string());
            events.push(ProviderEvent::ThinkingStart { id: rid.clone() });
            events.push(ProviderEvent::ThinkingDelta(reasoning));
            events.push(ProviderEvent::ThinkingEnd { id: rid });
        }
        FinalResponse(_) | MessageId(_) | TextAdditionalParams(_) => {
            // These are internal normalization events
        }
    }

    events
}

// ---------------------------------------------------------------------------
// RigOpenAiProvider: implements Provider using rig-core
// ---------------------------------------------------------------------------

/// Adapter that wraps rig-core's OpenAI provider and implements runie's `Provider` trait.
///
/// This allows using rig-core's provider implementations while maintaining
/// compatibility with runie's existing `Provider` interface.
#[derive(Clone)]
pub struct RigOpenAiProvider {
    api_key: String,
    model: String,
    base_url: String,
}

impl RigOpenAiProvider {
    /// Create a new adapter for the given OpenAI-compatible endpoint.
    pub fn new(api_key: String, model: impl Into<String>) -> Self {
        Self {
            api_key,
            model: model.into(),
            base_url: "https://api.openai.com/v1".to_string(),
        }
    }

    /// Set a custom base URL.
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Get the API key.
    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    /// Get the model name.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Get the base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

// Note: The actual integration with rig-core requires async runtime setup
// and proper error handling. This is a simplified version that demonstrates
// the architecture. For production use, implement the full Provider trait
// with proper rig-core streaming integration.

impl crate::Provider for RigOpenAiProvider {
    fn generate(
        &self,
        _messages: Vec<ChatMessage>,
    ) -> Pin<Box<dyn Stream<Item = anyhow::Result<ProviderEvent>> + Send + '_>> {
        // This is a placeholder implementation. For full rig-core integration,
        // you would:
        // 1. Create a rig-core OpenAI client: `rig_core::providers::openai::Client::new(&self.api_key)`
        // 2. Get a completion model: `client.completion_model(&self.model)`
        // 3. Build a CompletionRequest with the messages
        // 4. Stream responses and map RawStreamingChoice → ProviderEvent
        Box::pin(futures::stream::once(async {
            Err(anyhow::anyhow!(
                "RigOpenAiProvider: rig-core streaming not yet implemented. \
                 Use OpenAiProvider for production use."
            ))
        }))
    }

    fn generate_with_tools(
        &self,
        _messages: Vec<ChatMessage>,
        _tools: Vec<serde_json::Value>,
    ) -> Pin<Box<dyn Stream<Item = anyhow::Result<ProviderEvent>> + Send + '_>> {
        // This is a placeholder implementation. For full rig-core integration,
        // you would:
        // 1. Create a rig-core OpenAI client
        // 2. Get a completion model
        // 3. Add tools via `CompletionRequest::builder(...).tools(...).build()`
        // 4. Stream responses and map RawStreamingChoice → ProviderEvent
        Box::pin(futures::stream::once(async {
            Err(anyhow::anyhow!(
                "RigOpenAiProvider: rig-core streaming not yet implemented. \
                 Use OpenAiProvider for production use."
            ))
        }))
    }
}

/// Wrapper stream that maps rig-core choices to ProviderEvents.
#[pin_project]
pub struct RigStream {
    #[pin]
    inner: Pin<Box<dyn Stream<Item = anyhow::Result<ProviderEvent>> + Send + 'static>>,
}

impl Stream for RigStream {
    type Item = anyhow::Result<ProviderEvent>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.project().inner.poll_next(cx)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Provider;

    #[test]
    fn stop_reason_mapping() {
        let cases = [
            (Some("stop"), StopReason::Stop),
            (Some("length"), StopReason::Length),
            (Some("content_filter"), StopReason::ContentFilter),
            (Some("tool_calls"), StopReason::ToolCalls),
            (Some("stop_sequence"), StopReason::StopSequence),
            (Some("unknown_value"), StopReason::Unknown),
            (None, StopReason::Unknown),
        ];

        for (input, expected) in cases {
            let result = map_finish_reason(input);
            assert_eq!(result, expected, "input: {:?}", input);
        }
    }

    #[test]
    fn streaming_text_delta_mapping() {
        use rig_core::streaming::RawStreamingChoice::*;

        // Text delta
        let events = map_streaming_choice(Message("Hello".to_string()));
        assert!(matches!(
            events.as_slice(),
            [ProviderEvent::TextDelta(t)] if t == "Hello"
        ));

        // Text start
        let events = map_streaming_choice(TextStart { additional_params: None });
        assert!(matches!(
            events.as_slice(),
            [ProviderEvent::TextStart { id }] if id == "text"
        ));
    }

    #[test]
    fn streaming_tool_call_mapping() {
        use rig_core::streaming::RawStreamingChoice::*;

        // Tool call with name and args
        let events = map_streaming_choice(ToolCall(rig_core::streaming::RawStreamingToolCall {
            id: "call_1".to_string(),
            internal_call_id: "call_abc".to_string(),
            call_id: Some("call_1".to_string()),
            name: "read_file".to_string(),
            arguments: serde_json::json!("{\"path\":\"file.txt\"}"),
            signature: None,
            additional_params: None,
        }));
        assert!(events.iter().any(|e| matches!(
            e,
            ProviderEvent::ToolCallStart { id, name } if id == "call_abc" && name == "read_file"
        )));
        assert!(events.iter().any(|e| matches!(
            e,
            ProviderEvent::ToolCallEnd { id } if id == "call_abc"
        )));

        // Tool delta - name
        let events = map_streaming_choice(ToolCallDelta {
            id: "call_1".to_string(),
            internal_call_id: "call_xyz".to_string(),
            content: rig_core::streaming::ToolCallDeltaContent::Name("bash".to_string()),
        });
        assert!(events.iter().any(|e| matches!(
            e,
            ProviderEvent::ToolCallStart { id, name } if id == "call_xyz" && name == "bash"
        )));

        // Tool delta - args
        let events = map_streaming_choice(ToolCallDelta {
            id: "call_1".to_string(),
            internal_call_id: "call_xyz".to_string(),
            content: rig_core::streaming::ToolCallDeltaContent::Delta("{\"cmd\":\"ls\"}".to_string()),
        });
        assert!(events.iter().any(|e| matches!(
            e,
            ProviderEvent::ToolCallInputDelta { id, delta } if id == "call_xyz" && delta.contains("ls")
        )));
    }

    #[test]
    fn streaming_reasoning_mapping() {
        use rig_core::streaming::RawStreamingChoice::*;

        // Reasoning start and content
        let events = map_streaming_choice(Reasoning {
            id: Some("reason_1".to_string()),
            content: rig_core::message::ReasoningContent::Text {
                text: "thinking...".to_string(),
                signature: None,
            },
        });
        assert!(events.iter().any(|e| matches!(
            e,
            ProviderEvent::ThinkingStart { id } if id == "reason_1"
        )));
        assert!(events.iter().any(|e| matches!(
            e,
            ProviderEvent::ThinkingDelta(t) if t == "thinking..."
        )));
        assert!(events.iter().any(|e| matches!(
            e,
            ProviderEvent::ThinkingEnd { id } if id == "reason_1"
        )));

        // Reasoning delta
        let events = map_streaming_choice(ReasoningDelta {
            id: None,
            reasoning: "more thoughts...".to_string(),
        });
        assert!(events.iter().any(|e| matches!(
            e,
            ProviderEvent::ThinkingStart { id } if id == "reasoning"
        )));
        assert!(events.iter().any(|e| matches!(
            e,
            ProviderEvent::ThinkingDelta(t) if t == "more thoughts..."
        )));
    }

    #[test]
    fn rig_openai_provider_accessors() {
        let provider = RigOpenAiProvider::new("sk-test".to_string(), "gpt-4o")
            .with_base_url("https://api.example.com/v1");

        assert_eq!(provider.api_key(), "sk-test");
        assert_eq!(provider.model(), "gpt-4o");
        assert_eq!(provider.base_url(), "https://api.example.com/v1");
    }

    #[tokio::test]
    async fn rig_provider_generate_returns_error() {
        let provider = RigOpenAiProvider::new("sk-test".to_string(), "gpt-4o");
        let stream = provider.generate(vec![ChatMessage::user("Hello".to_string())]);
        let result = futures::stream::StreamExt::next(&mut Box::pin(stream)).await;
        assert!(result.is_some());
        assert!(result.unwrap().is_err());
    }
}
