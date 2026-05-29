//! Session adapter using rig-core's InMemoryConversationMemory.
//!
//! This module provides an adapter that bridges runie's Session types
//! with rig-core's ConversationMemory trait. It allows using rig-core's
//! built-in memory management while maintaining compatibility with runie's
//! message format.
//!
//! ## Architecture
//!
//! ```text
//! runie-core          runie-ai              rig-core
//!    │                   │                     │
//!    │   Vec<Message>    │                     │
//!    ├──────────────────►│                     │
//!    │                   │  convert_messages  │
//!    │                   ├────────────────────►│
//!    │                   │                     │
//!    │                   │  InMemoryConversationMemory
//!    │                   │                     │
//!    │   Vec<Message>    │   convert_rig_to_runie
//!    │◄──────────────────┤                     │
//!    │                   │                     │
//! ```
//!
//! ## Usage
//!
//! ```ignore
//! use runie_ai::session_adapter::RigSessionAdapter;
//! use runie_core::Message;
//!
//! let adapter = RigSessionAdapter::new();
//!
//! // Save messages to a conversation
//! adapter.save("session-1", vec![Message::user("Hello")]).await;
//!
//! // Load messages from a conversation
//! let messages = adapter.load("session-1").await;
//! ```
//!
//! ## Migration Path
//!
//! The eventual goal is to replace the custom Session/MessageNode tree in
//! runie-core with direct use of rig-core's memory system. This adapter
//! provides a transition layer that:
//! 1. Uses rig-core's InMemoryConversationMemory as the backing store
//! 2. Maintains runie-core's Message type for API compatibility
//! 3. Enables gradual migration of code that depends on Session types
//!
//! Once migration is complete, code can directly use rig-core's
//! ConversationMemory trait without the adapter layer.

use rig_core::memory::{ConversationMemory, InMemoryConversationMemory};
use rig_core::completion::message::{
    AssistantContent, Message as RigMessage, Reasoning, Text, ToolCall as RigToolCall,
    ToolFunction, ToolResult, ToolResultContent, UserContent,
};
use rig_core::OneOrMany;
use runie_core::Message;
use thiserror::Error;

/// Errors that can occur when using the session adapter.
#[derive(Debug, Error)]
pub enum SessionAdapterError {
    #[error("memory error: {0}")]
    Memory(#[from] rig_core::memory::MemoryError),
    #[error("conversion error: {0}")]
    Conversion(String),
}

/// Adapter that wraps rig-core's InMemoryConversationMemory to work with
/// runie's Message type.
///
/// This adapter provides a simple interface for persisting and retrieving
/// conversations using rig-core's built-in in-memory storage. It handles
/// conversion between runie::Message and rig_core::completion::Message types.
#[derive(Clone)]
pub struct RigSessionAdapter {
    memory: InMemoryConversationMemory,
}

impl RigSessionAdapter {
    /// Creates a new session adapter with an empty in-memory store.
    pub fn new() -> Self {
        Self {
            memory: InMemoryConversationMemory::new(),
        }
    }

    /// Creates a new session adapter with a message filter applied to all loads.
    ///
    /// The filter transforms the message list after loading from storage.
    /// This is useful for implementing sliding window truncation, summarization,
    /// or other history-shaping policies.
    pub fn with_filter<F>(filter: F) -> Self
    where
        F: Fn(Vec<Message>) -> Vec<Message> + Send + Sync + 'static,
    {
        let memory = InMemoryConversationMemory::new().with_filter(move |msgs| {
            let runie_messages: Vec<Message> = msgs
                .into_iter()
                .filter_map(convert_rig_to_runie_single)
                .collect();
            let filtered = filter(runie_messages);
            filtered.into_iter().map(convert_runie_to_rig).collect()
        });
        Self { memory }
    }

    /// Loads all messages for a conversation.
    ///
    /// Returns an empty vector if the conversation has no stored messages.
    /// Messages are returned in chronological order (oldest first).
    pub async fn load(&self, conversation_id: &str) -> Result<Vec<Message>, SessionAdapterError> {
        let rig_messages = self.memory.load(conversation_id).await?;
        let messages = rig_messages
            .into_iter()
            .filter_map(convert_rig_to_runie_single)
            .collect();
        Ok(messages)
    }

    /// Appends messages to a conversation.
    ///
    /// Messages are appended to the existing conversation history.
    /// If the conversation doesn't exist, it is created.
    pub async fn save(
        &self,
        conversation_id: &str,
        messages: Vec<Message>,
    ) -> Result<(), SessionAdapterError> {
        let rig_messages: Vec<RigMessage> = messages.into_iter().map(convert_runie_to_rig).collect();
        self.memory.append(conversation_id, rig_messages).await?;
        Ok(())
    }

    /// Clears all messages for a conversation.
    pub async fn clear(&self, conversation_id: &str) -> Result<(), SessionAdapterError> {
        self.memory.clear(conversation_id).await?;
        Ok(())
    }

    /// Returns the underlying rig-core memory for advanced usage.
    ///
    /// This is intended for cases where you need direct access to rig-core's
    /// ConversationMemory trait, such as when passing it to rig agents.
    pub fn memory(&self) -> &InMemoryConversationMemory {
        &self.memory
    }
}

impl Default for RigSessionAdapter {
    fn default() -> Self {
        Self::new()
    }
}

/// Converts a single rig Message to a runie Message.
///
/// Returns None if the message type cannot be converted (e.g., images, audio).
fn convert_rig_to_runie_single(rig_msg: RigMessage) -> Option<Message> {
    match rig_msg {
        RigMessage::System { content } => Some(Message::System { content }),
        RigMessage::User { content } => {
            let text = extract_text_from_user_content(&content)?;
            let attachments = extract_attachments_from_user_content(&content);
            Some(Message::User {
                content: text,
                attachments,
            })
        }
        RigMessage::Assistant { content, .. } => {
            let (text, tool_calls, thinking) = extract_assistant_content(&content);
            Some(Message::Assistant {
                content: text,
                tool_calls,
                thinking,
            })
        }
    }
}

/// Extracts text content from UserContent, returning None if no text is present.
fn extract_text_from_user_content(content: &OneOrMany<UserContent>) -> Option<String> {
    let texts: Vec<&str> = content
        .iter()
        .filter_map(|c| {
            if let UserContent::Text(Text { text }) = c {
                Some(text.as_str())
            } else {
                None
            }
        })
        .collect();

    if texts.is_empty() {
        None
    } else {
        Some(texts.join("\n"))
    }
}

/// Extracts attachment data from UserContent.
///
/// Currently returns empty vec since runie-core's Attachment type
/// doesn't directly map from rig content types.
fn extract_attachments_from_user_content(_content: &OneOrMany<UserContent>) -> Vec<runie_core::Attachment> {
    // NOTE: Future enhancement — implement attachment extraction when runie's Attachment type
    // is updated to support the same content types as rig (images, audio, etc.)
    Vec::new()
}

/// Extracts text, tool calls, and thinking from AssistantContent.
fn extract_assistant_content(
    content: &OneOrMany<AssistantContent>,
) -> (String, Vec<runie_core::ToolCall>, Option<String>) {
    let mut text = String::new();
    let mut tool_calls = Vec::new();
    let mut thinking = None;

    for item in content.iter() {
        match item {
            AssistantContent::Text(t) => {
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(&t.text);
            }
            AssistantContent::ToolCall(tc) => {
                tool_calls.push(runie_core::ToolCall {
                    id: tc.id.clone(),
                    name: tc.function.name.clone(),
                    arguments: tc.function.arguments.clone(),
                });
            }
            AssistantContent::Reasoning(r) => {
                thinking = Some(r.display_text());
            }
            AssistantContent::Image(_) => {
                // NOTE: Future enhancement — handle assistant images when runie supports them
            }
        }
    }

    (text, tool_calls, thinking)
}

/// Converts a runie Message to a rig Message.
fn convert_runie_to_rig(msg: Message) -> RigMessage {
    match msg {
        Message::System { content } => RigMessage::System { content },
        Message::User { content, attachments } => convert_user_to_rig(content, attachments),
        Message::Assistant { content, tool_calls, thinking } => convert_assistant_to_rig(content, tool_calls, thinking),
        Message::ToolResult { tool_call_id, content, is_error: _ } => convert_tool_result_to_rig(tool_call_id, content),
    }
}

fn convert_user_to_rig(content: String, _attachments: Vec<runie_core::Attachment>) -> RigMessage {
    // NOTE: attachments not yet supported - use text content only
    RigMessage::User {
        content: OneOrMany::one(UserContent::Text(Text { text: content })),
    }
}

fn convert_assistant_to_rig(content: String, tool_calls: Vec<runie_core::ToolCall>, thinking: Option<String>) -> RigMessage {
    let mut contents = Vec::new();

    if !content.is_empty() {
        contents.push(AssistantContent::Text(Text { text: content }));
    }

    for tc in tool_calls {
        contents.push(AssistantContent::ToolCall(RigToolCall {
            id: tc.id,
            call_id: None,
            function: ToolFunction {
                name: tc.name,
                arguments: tc.arguments,
            },
            signature: None,
            additional_params: None,
        }));
    }

    // Convert thinking to rig's Reasoning content
    if let Some(thinking_text) = thinking {
        contents.push(AssistantContent::Reasoning(Reasoning::new(&thinking_text)));
    }

    // If assistant message has no content, add empty text
    if contents.is_empty() {
        contents.push(AssistantContent::Text(Text { text: String::new() }));
    }

    let content = OneOrMany::many(contents).unwrap_or_else(|_| {
        OneOrMany::one(AssistantContent::Text(Text { text: String::new() }))
    });

    RigMessage::Assistant { id: None, content }
}

fn convert_tool_result_to_rig(tool_call_id: String, content: String) -> RigMessage {
    RigMessage::User {
        content: OneOrMany::one(UserContent::ToolResult(ToolResult {
            id: tool_call_id,
            call_id: None,
            content: OneOrMany::one(ToolResultContent::Text(Text { text: content })),
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn round_trip_user_message() {
        let adapter = RigSessionAdapter::new();
        let original = Message::User {
            content: "Hello, world!".to_string(),
            attachments: Vec::new(),
        };

        adapter
            .save("test-1", vec![original.clone()])
            .await
            .unwrap();

        let loaded = adapter.load("test-1").await.unwrap();
        assert_eq!(loaded.len(), 1);

        match (&original, &loaded[0]) {
            (
                Message::User { content: c1, .. },
                Message::User { content: c2, .. },
            ) => assert_eq!(c1, c2),
            _ => panic!("Message type mismatch"),
        }
    }

    fn create_test_assistant_message() -> Message {
        Message::Assistant {
            content: "I'm an assistant".to_string(),
            tool_calls: vec![runie_core::ToolCall {
                id: "call-1".to_string(),
                name: "test_tool".to_string(),
                arguments: serde_json::json!({"arg": "value"}),
            }],
            thinking: Some("Let me think...".to_string()),
        }
    }

    fn verify_assistant_messages_match(loaded: &Message, original: &Message) {
        match (loaded, original) {
            (
                Message::Assistant {
                    content: c2,
                    tool_calls: tc2,
                    thinking: t2,
                },
                Message::Assistant {
                    content: c1,
                    tool_calls: tc1,
                    thinking: t1,
                },
            ) => {
                assert_eq!(c1, c2);
                assert_eq!(tc1.len(), tc2.len());
                if !tc1.is_empty() {
                    assert_eq!(tc1[0].id, tc2[0].id);
                    assert_eq!(tc1[0].name, tc2[0].name);
                }
                assert!(t2.is_some() || t1.is_none());
            }
            _ => panic!("Message type mismatch"),
        }
    }

    #[tokio::test]
    async fn round_trip_assistant_message() {
        let adapter = RigSessionAdapter::new();
        let original = create_test_assistant_message();

        adapter
            .save("test-2", vec![original.clone()])
            .await
            .unwrap();

        let loaded = adapter.load("test-2").await.unwrap();
        assert_eq!(loaded.len(), 1);

        verify_assistant_messages_match(&loaded[0], &original);
    }

    #[tokio::test]
    async fn load_empty_conversation() {
        let adapter = RigSessionAdapter::new();
        let loaded = adapter.load("nonexistent").await.unwrap();
        assert!(loaded.is_empty());
    }

    #[tokio::test]
    async fn clear_conversation() {
        let adapter = RigSessionAdapter::new();
        adapter
            .save(
                "test-3",
                vec![Message::User {
                    content: "test".to_string(),
                    attachments: Vec::new(),
                }],
            )
            .await
            .unwrap();

        adapter.clear("test-3").await.unwrap();

        let loaded = adapter.load("test-3").await.unwrap();
        assert!(loaded.is_empty());
    }

    #[tokio::test]
    async fn multiple_messages_per_conversation() {
        let adapter = RigSessionAdapter::new();

        let messages = vec![
            Message::System {
                content: "You are a helpful assistant.".to_string(),
            },
            Message::User {
                content: "Hi!".to_string(),
                attachments: Vec::new(),
            },
            Message::Assistant {
                content: "Hello! How can I help you?".to_string(),
                tool_calls: Vec::new(),
                thinking: None,
            },
        ];

        adapter
            .save("test-4", messages.clone())
            .await
            .unwrap();

        let loaded = adapter.load("test-4").await.unwrap();
        assert_eq!(loaded.len(), 3);
    }
}
