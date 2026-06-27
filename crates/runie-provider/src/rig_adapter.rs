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
//! └── event mapping — rig StreamedAssistantContent → ProviderEvent
//! ```
//!
//! ## Usage
//!
//! The `RigOpenAiProvider` can be used as a drop-in replacement for the
//! custom `OpenAiProvider` implementation.
//!
//! ## Status
//!
//! Phase 1 (Foundation): Complete - event mapping infrastructure is in place.
//! Phase 2 (Full Migration): TODO - streaming integration pending rig-core API compatibility.

#![allow(dead_code)] // Placeholder for future streaming implementation

use futures::Stream;
use rig_core::completion::message::{
    AssistantContent, Message, ReasoningContent, Text as RigText, ToolCall as RigToolCall,
    ToolFunction, ToolResult, ToolResultContent, UserContent,
};
use rig_core::providers::openai::completion::streaming::StreamingCompletionResponse;
use rig_core::streaming::StreamedAssistantContent;
use rig_core::OneOrMany;
use runie_core::message::ChatMessage;
use runie_core::provider_event::ProviderEvent;
use std::pin::Pin;

// ---------------------------------------------------------------------------
// Event mapping: rig-core StreamedAssistantContent → ProviderEvent
// ---------------------------------------------------------------------------

/// Maps rig-core streaming content to one or more runie ProviderEvents.
fn map_streamed_content(
    content: StreamedAssistantContent<StreamingCompletionResponse>,
) -> Vec<ProviderEvent> {
    use StreamedAssistantContent::*;
    let mut events = Vec::new();

    match content {
        Text(text) => {
            events.push(ProviderEvent::TextStart { id: "text".to_string() });
            events.push(ProviderEvent::TextDelta(text.text));
        }
        ToolCall { tool_call, internal_call_id } => {
            events.push(ProviderEvent::ToolCallStart {
                id: internal_call_id.clone(),
                name: tool_call.function.name.clone(),
            });
            // Parse arguments from the tool call
            if let Ok(args) = serde_json::to_string(&tool_call.function.arguments) {
                if !args.is_empty() && args != "null" {
                    events.push(ProviderEvent::ToolCallInputDelta {
                        id: internal_call_id.clone(),
                        delta: args,
                    });
                }
            }
            events.push(ProviderEvent::ToolCallEnd { id: internal_call_id });
        }
        ToolCallDelta { id: _, internal_call_id, content } => {
            match content {
                rig_core::streaming::ToolCallDeltaContent::Name(name) => {
                    events.push(ProviderEvent::ToolCallStart {
                        id: internal_call_id,
                        name,
                    });
                }
                rig_core::streaming::ToolCallDeltaContent::Delta(args) => {
                    events.push(ProviderEvent::ToolCallInputDelta {
                        id: internal_call_id,
                        delta: args,
                    });
                }
            }
        }
        Reasoning(reasoning) => {
            let rid = reasoning.id.clone().unwrap_or_else(|| "reasoning".to_string());
            events.push(ProviderEvent::ThinkingStart { id: rid.clone() });
            // Extract text from reasoning content blocks
            for block in &reasoning.content {
                if let ReasoningContent::Text { text, .. } = block {
                    events.push(ProviderEvent::ThinkingDelta(text.clone()));
                }
            }
            events.push(ProviderEvent::ThinkingEnd { id: rid });
        }
        ReasoningDelta { id, reasoning } => {
            let rid = id.unwrap_or_else(|| "reasoning".to_string());
            events.push(ProviderEvent::ThinkingStart { id: rid.clone() });
            events.push(ProviderEvent::ThinkingDelta(reasoning));
            events.push(ProviderEvent::ThinkingEnd { id: rid });
        }
        Final(_) => {
            // Final event - handled by the Finish event emission
        }
    }

    events
}

// ---------------------------------------------------------------------------
// Message conversion: ChatMessage → rig-core Message
// ---------------------------------------------------------------------------

/// Convert a runie ChatMessage to a rig-core completion Message.
fn chat_message_to_rig(msg: &ChatMessage) -> Option<Message> {
    match msg.role.as_str() {
        "system" => {
            let text = msg.content();
            Some(Message::System { content: text })
        }
        "user" => {
            let text = msg.content();
            Some(Message::User {
                content: OneOrMany::one(UserContent::Text(RigText::new(text))),
            })
        }
        "assistant" => {
            let text = msg.content();
            let tool_calls: Vec<RigToolCall> = msg
                .tool_calls()
                .iter()
                .map(|tc| RigToolCall {
                    id: tc.id.clone(),
                    call_id: None,
                    function: ToolFunction {
                        name: tc.name.clone(),
                        arguments: tc.args.clone(),
                    },
                    additional_params: None,
                    signature: None,
                })
                .collect();

            let mut content_items = Vec::new();
            if !text.is_empty() {
                content_items.push(AssistantContent::Text(RigText::new(text)));
            }
            for tc in tool_calls {
                content_items.push(AssistantContent::ToolCall(tc));
            }

            // Create content with OneOrMany
            let content = if content_items.is_empty() {
                OneOrMany::one(AssistantContent::Text(RigText::new(String::new())))
            } else {
                OneOrMany::one(content_items.pop().unwrap_or(AssistantContent::Text(RigText::new(String::new()))))
            };

            Some(Message::Assistant { id: None, content })
        }
        "tool" => {
            let text = msg.content();
            let tool_call_id = msg.tool_call_id.clone().unwrap_or_default();
            // Tool results are represented as User messages with ToolResult content
            Some(Message::User {
                content: OneOrMany::one(UserContent::ToolResult(ToolResult {
                    id: tool_call_id,
                    call_id: None,
                    content: OneOrMany::one(ToolResultContent::Text(RigText::new(text))),
                })),
            })
        }
        _ => None,
    }
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
    tools: Vec<serde_json::Value>,
}

impl RigOpenAiProvider {
    /// Create a new adapter for the given OpenAI-compatible endpoint.
    pub fn new(api_key: String, model: impl Into<String>) -> Self {
        Self {
            api_key,
            model: model.into(),
            base_url: "https://api.openai.com/v1".to_string(),
            tools: Vec::new(),
        }
    }

    /// Set a custom base URL.
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Set tools for this provider.
    pub fn with_tools(mut self, tools: Vec<serde_json::Value>) -> Self {
        self.tools = tools;
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

    /// Convert ChatMessages to rig-core messages.
    fn build_messages(&self, messages: &[ChatMessage]) -> Vec<Message> {
        messages.iter().filter_map(chat_message_to_rig).collect()
    }
}

impl crate::Provider for RigOpenAiProvider {
    fn generate(
        &self,
        _messages: Vec<ChatMessage>,
    ) -> Pin<Box<dyn Stream<Item = anyhow::Result<ProviderEvent>> + Send + '_>> {
        // TODO: Full rig-core streaming integration pending Phase 2 completion.
        // See crates/runie-provider/src/rig_adapter.rs for event mapping infrastructure.
        Box::pin(async_stream::stream! {
            yield Err(anyhow::anyhow!(
                "RigOpenAiProvider::generate - rig-core streaming integration pending (see task: unify-provider-stack-with-rig-core)"
            ));
        })
    }

    fn generate_with_tools(
        &self,
        messages: Vec<ChatMessage>,
        _tools: Vec<serde_json::Value>,
    ) -> Pin<Box<dyn Stream<Item = anyhow::Result<ProviderEvent>> + Send + '_>> {
        // Delegate to generate for now - full tool support requires more setup
        self.generate(messages)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streaming_text_mapping() {
        use rig_core::message::Text;

        // Text content - use Default for additional_params
        let events = map_streamed_content(StreamedAssistantContent::Text(Text {
            text: "Hello".to_string(),
            additional_params: Default::default(),
        }));
        assert!(matches!(
            events.as_slice(),
            [ProviderEvent::TextStart { id }, ProviderEvent::TextDelta(t)]
            if id == "text" && t == "Hello"
        ));
    }

    #[test]
    fn streaming_tool_call_mapping() {
        use rig_core::completion::message::ToolCall;

        // Tool call
        let events = map_streamed_content(StreamedAssistantContent::ToolCall {
            tool_call: ToolCall {
                id: "call_1".to_string(),
                call_id: None,
                function: rig_core::completion::message::ToolFunction {
                    name: "read_file".to_string(),
                    arguments: serde_json::json!({"path": "file.txt"}),
                },
                additional_params: None,
                signature: None,
            },
            internal_call_id: "call_abc".to_string(),
        });
        assert!(events.iter().any(|e| matches!(
            e,
            ProviderEvent::ToolCallStart { id, name } if id == "call_abc" && name == "read_file"
        )));
        assert!(events.iter().any(|e| matches!(
            e,
            ProviderEvent::ToolCallEnd { id } if id == "call_abc"
        )));
    }

    #[test]
    fn streaming_tool_call_delta_name() {
        use rig_core::streaming::ToolCallDeltaContent;

        let events = map_streamed_content(StreamedAssistantContent::ToolCallDelta {
            id: "call_1".to_string(),
            internal_call_id: "call_xyz".to_string(),
            content: ToolCallDeltaContent::Name("bash".to_string()),
        });
        assert!(events.iter().any(|e| matches!(
            e,
            ProviderEvent::ToolCallStart { id, name } if id == "call_xyz" && name == "bash"
        )));
    }

    #[test]
    fn streaming_tool_call_delta_args() {
        use rig_core::streaming::ToolCallDeltaContent;

        let events = map_streamed_content(StreamedAssistantContent::ToolCallDelta {
            id: "call_1".to_string(),
            internal_call_id: "call_xyz".to_string(),
            content: ToolCallDeltaContent::Delta("{\"cmd\":\"ls\"}".to_string()),
        });
        assert!(events.iter().any(|e| matches!(
            e,
            ProviderEvent::ToolCallInputDelta { id, delta } if id == "call_xyz" && delta.contains("ls")
        )));
    }

    #[test]
    fn streaming_reasoning_mapping() {
        // Test ReasoningDelta directly since Reasoning struct is non-exhaustive
        let events = map_streamed_content(StreamedAssistantContent::ReasoningDelta {
            id: Some("reason_1".to_string()),
            reasoning: "thinking...".to_string(),
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
    }

    #[test]
    fn streaming_final_mapping() {
        // Final event should produce no additional events
        // Use correct Usage type from the openai provider
        let events = map_streamed_content(StreamedAssistantContent::Final(
            rig_core::providers::openai::completion::streaming::StreamingCompletionResponse {
                usage: rig_core::providers::openai::Usage::default(),
            },
        ));
        assert!(events.is_empty());
    }

    #[test]
    fn rig_openai_provider_accessors() {
        let provider = RigOpenAiProvider::new("sk-test".to_string(), "gpt-4o")
            .with_base_url("https://api.example.com/v1");

        assert_eq!(provider.api_key(), "sk-test");
        assert_eq!(provider.model(), "gpt-4o");
        assert_eq!(provider.base_url(), "https://api.example.com/v1");
    }

    #[test]
    fn chat_message_conversion_system() {
        let msg = ChatMessage::system("You are a helpful assistant".to_string());
        let rig_msg = chat_message_to_rig(&msg);
        assert!(rig_msg.is_some());
        if let Some(Message::System { content }) = rig_msg {
            assert!(content.contains("helpful"));
        } else {
            panic!("Expected System message");
        }
    }

    #[test]
    fn chat_message_conversion_user() {
        let msg = ChatMessage::user("Hello, world!".to_string());
        let rig_msg = chat_message_to_rig(&msg);
        assert!(rig_msg.is_some());
        // Just verify it doesn't panic
    }

    #[test]
    fn chat_message_conversion_assistant_with_text() {
        let msg = ChatMessage::assistant("Hello!".to_string());
        let rig_msg = chat_message_to_rig(&msg);
        assert!(rig_msg.is_some());
    }

    #[test]
    fn chat_message_conversion_tool() {
        // Create a tool message directly with tool_call_id
        let msg = ChatMessage {
            role: runie_core::message::Role::Tool,
            timestamp: 0.0,
            id: "tool_result".to_string(),
            provider: String::new(),
            metadata: Default::default(),
            tool_call_id: Some("call_abc".to_string()),
            provider_metadata: None,
            parts: vec![runie_core::message::Part::Text {
                content: "File contents here".to_string(),
            }],
        };

        let rig_msg = chat_message_to_rig(&msg);
        assert!(rig_msg.is_some());
    }

    #[test]
    fn build_messages_converts_all_types() {
        let messages = vec![
            ChatMessage::system("You are a helpful assistant".to_string()),
            ChatMessage::user("Hello!".to_string()),
        ];
        let provider = RigOpenAiProvider::new("sk-test".to_string(), "gpt-4o");
        let rig_messages = provider.build_messages(&messages);
        assert_eq!(rig_messages.len(), 2);
    }
}
