//! Feed - single source of truth for conversation feed state.
//!
//! Key invariants:
//! - UserMessage always precedes AssistantMessage (never follows)
//! - AssistantMessage contains its thoughts and tool_calls inline
//! - Thoughts/ToolCalls are NOT separate feed items (they attach to assistant)
//! - Streaming content appended in-place via append_to_last

use std::collections::HashSet;
use uuid::Uuid;

pub mod builder;

#[derive(Debug, Clone, PartialEq)]
pub struct Thought {
    pub duration: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolCall {
    pub name: String,
    pub args: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FeedItem {
    UserMessage { id: String, text: String, timestamp: Option<String> },
    AssistantMessage {
        id: String,
        text: String,
        thoughts: Vec<Thought>,
        tool_calls: Vec<ToolCall>,
        timestamp: Option<String>,
        turn_duration: Option<f32>,
    },
    SystemNotice { text: String },
}

impl FeedItem {
    pub fn id(&self) -> &str {
        match self {
            FeedItem::UserMessage { id, .. } => id,
            FeedItem::AssistantMessage { id, .. } => id,
            FeedItem::SystemNotice { .. } => "",
        }
    }

    pub fn is_message(&self) -> bool {
        matches!(self, FeedItem::UserMessage { .. } | FeedItem::AssistantMessage { .. })
    }
}

/// Feed - single source of truth for feed state.
///
/// Maintains valid conversation structure:
/// - Messages in order: User → Assistant → User → Assistant...
/// - Thoughts and ToolCalls attached to AssistantMessage, not separate items
#[derive(Debug, Clone, PartialEq)]
pub struct Feed {
    items: Vec<FeedItem>,
    /// IDs for deduplication
    seen_ids: HashSet<String>,
}

impl Default for Feed {
    fn default() -> Self {
        Self::new()
    }
}

impl Feed {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            seen_ids: HashSet::new(),
        }
    }

    /// Create a new FeedBuilder for declarative feed construction.
    pub fn builder() -> crate::components::message_list::feed::builder::FeedBuilder {
        crate::components::message_list::feed::builder::FeedBuilder::new()
    }

    /// Add a user message. Returns mutable reference to the inserted item.
    pub fn add_user_message(&mut self, text: String) -> &mut FeedItem {
        let id = Uuid::new_v4().to_string();
        self.items.push(FeedItem::UserMessage {
            id: id.clone(),
            text,
            timestamp: None,
        });
        self.seen_ids.insert(id);
        let idx = self.items.len().saturating_sub(1);
        &mut self.items[idx]
    }

    /// Begin an assistant message. Returns mutable reference to the inserted item.
    pub fn add_assistant_message(&mut self) -> &mut FeedItem {
        let id = Uuid::new_v4().to_string();
        self.items.push(FeedItem::AssistantMessage {
            id: id.clone(),
            text: String::new(),
            thoughts: Vec::new(),
            tool_calls: Vec::new(),
            timestamp: None,
            turn_duration: None,
        });
        self.seen_ids.insert(id);
        let idx = self.items.len().saturating_sub(1);
        &mut self.items[idx]
    }

    /// Append text to the last assistant message.
    /// For streaming content updates.
    /// Note: Only appends to AssistantMessage, not UserMessage (user messages are immutable).
    pub fn append_to_last(&mut self, text: &str) {
        if let Some(last) = self.items.last_mut() {
            if let FeedItem::AssistantMessage { text: ref mut t, .. } = last {
                t.push_str(text);
            }
        }
    }

    /// Update text of last assistant message (full replacement, for streaming).
    pub fn update_last_assistant_text(&mut self, text: &str) {
        if let Some(FeedItem::AssistantMessage { text: ref mut t, .. }) = self.items.last_mut() {
            *t = text.to_string();
        }
    }

    /// Add a thought to the last assistant message.
    /// Silently no-op if no assistant message exists.
    pub fn add_thought(&mut self, duration: f32) {
        if let Some(FeedItem::AssistantMessage { thoughts, .. }) = self.items.last_mut() {
            thoughts.push(Thought { duration });
        }
    }

    /// Add a tool call to the last assistant message.
    /// Silently no-op if no assistant message exists.
    pub fn add_tool_call(&mut self, name: String, args: String) {
        if let Some(FeedItem::AssistantMessage { tool_calls, .. }) = self.items.last_mut() {
            tool_calls.push(ToolCall { name, args });
        }
    }

    /// Complete the current turn with duration.
    /// Sets turn_duration on the last assistant message.
    pub fn complete_turn(&mut self, duration: f32) {
        if let Some(FeedItem::AssistantMessage { turn_duration, .. }) = self.items.last_mut() {
            *turn_duration = Some(duration);
        }
    }

    /// Get all feed items.
    pub fn items(&self) -> &[FeedItem] {
        &self.items
    }

    /// Get mutable reference to items.
    pub fn items_mut(&mut self) -> &mut Vec<FeedItem> {
        &mut self.items
    }

    /// Clear all items.
    pub fn clear(&mut self) {
        self.items.clear();
        self.seen_ids.clear();
    }

    /// Check if an ID has been seen (for deduplication).
    pub fn has_id(&self, id: &str) -> bool {
        self.seen_ids.contains(id)
    }

    /// Add an item only if ID hasn't been seen (dedup by ID).
    /// Returns true if item was added.
    pub fn add_if_new(&mut self, item: FeedItem) -> bool {
        let id = item.id();
        if id.is_empty() || !self.seen_ids.contains(id) {
            if !id.is_empty() {
                self.seen_ids.insert(id.to_string());
            }
            self.items.push(item);
            true
        } else {
            false
        }
    }

    /// Check if last item is an assistant message (for streaming check).
    pub fn has_assistant_in_progress(&self) -> bool {
        matches!(self.items.last(), Some(FeedItem::AssistantMessage { .. }))
    }

    /// Get last assistant message if exists.
    pub fn last_assistant_mut(&mut self) -> Option<&mut FeedItem> {
        if let Some(idx) = self.items.iter().rposition(|i| matches!(i, FeedItem::AssistantMessage { .. })) {
            // Return only if it's the last item
            if idx == self.items.len() - 1 {
                return self.items.get_mut(idx);
            }
        }
        None
    }

    /// Get the count of items.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Add a system notice.
    pub fn add_system_notice(&mut self, text: String) {
        self.items.push(FeedItem::SystemNotice { text });
    }

    /// Remove last item if it's an assistant with empty text.
    pub fn remove_last_empty_assistant(&mut self) {
        if let Some(FeedItem::AssistantMessage { text, thoughts, tool_calls, .. }) = self.items.last() {
            if text.is_empty() && thoughts.is_empty() && tool_calls.is_empty() {
                self.items.pop();
            }
        }
    }
}

/// Convert MessageItem to FeedItem for rendering pipeline migration.
/// Note: Thought and ToolCall items are filtered out (now inline in AssistantMessage).
/// Separator items are also filtered (turn timing shown via turn_duration).
impl TryFrom<crate::components::message_list::MessageItem> for FeedItem {
    type Error = ();

    fn try_from(item: crate::components::message_list::MessageItem) -> Result<Self, Self::Error> {
        use crate::components::message_list::MessageItem::*;
        match item {
            User { text, timestamp, .. } => Ok(FeedItem::UserMessage {
                id: Uuid::new_v4().to_string(),
                text,
                timestamp,
            }),
            Assistant { text, timestamp, .. } => Ok(FeedItem::AssistantMessage {
                id: Uuid::new_v4().to_string(),
                text,
                thoughts: Vec::new(),
                tool_calls: Vec::new(),
                timestamp,
                turn_duration: None,
            }),
            System { text } => Ok(FeedItem::SystemNotice { text }),
            // Filter out items now inline in AssistantMessage or UI-only
            Thought { .. } | ToolCall { .. } | Separator { .. } | Edit { .. }
            | Error { .. } | ToolRunning { .. } | ToolComplete { .. }
            | PlanStep { .. } | Interrupt | Rewind { .. } => Err(()),
        }
    }
}

impl From<Vec<crate::components::message_list::MessageItem>> for Feed {
    fn from(messages: Vec<crate::components::message_list::MessageItem>) -> Self {
        let mut feed = Feed::new();
        for item in messages {
            if let Ok(feed_item) = FeedItem::try_from(item) {
                feed.items.push(feed_item);
            }
        }
        feed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_user_message() {
        let mut feed = Feed::new();
        feed.add_user_message("Hello".to_string());
        assert_eq!(feed.items.len(), 1);
        assert!(matches!(feed.items[0], FeedItem::UserMessage { .. }));
    }

    #[test]
    fn test_add_assistant_message() {
        let mut feed = Feed::new();
        feed.add_user_message("Hello".to_string());
        feed.add_assistant_message();
        assert_eq!(feed.items.len(), 2);
        assert!(matches!(feed.items[1], FeedItem::AssistantMessage { .. }));
    }

    #[test]
    fn test_thoughts_attached_to_assistant() {
        let mut feed = Feed::new();
        feed.add_user_message("Hello".to_string());
        feed.add_assistant_message();
        feed.add_thought(1.5);
        feed.add_thought(2.0);
        match &feed.items[1] {
            FeedItem::AssistantMessage { thoughts, .. } => {
                assert_eq!(thoughts.len(), 2);
                assert_eq!(thoughts[0].duration, 1.5);
                assert_eq!(thoughts[1].duration, 2.0);
            }
            _ => panic!("Expected AssistantMessage"),
        }
    }

    #[test]
    fn test_tool_calls_attached_to_assistant() {
        let mut feed = Feed::new();
        feed.add_user_message("Hello".to_string());
        feed.add_assistant_message();
        feed.add_tool_call("bash".to_string(), "pwd".to_string());
        match &feed.items[1] {
            FeedItem::AssistantMessage { tool_calls, .. } => {
                assert_eq!(tool_calls.len(), 1);
                assert_eq!(tool_calls[0].name, "bash");
            }
            _ => panic!("Expected AssistantMessage"),
        }
    }

    #[test]
    fn test_append_to_last() {
        let mut feed = Feed::new();
        feed.add_user_message("Hello".to_string());
        feed.add_assistant_message();
        feed.append_to_last("Hi");
        match &feed.items[1] {
            FeedItem::AssistantMessage { text, .. } => {
                assert_eq!(text, "Hi");
            }
            _ => panic!("Expected AssistantMessage"),
        }
    }

    #[test]
    fn test_complete_turn() {
        let mut feed = Feed::new();
        feed.add_user_message("Hello".to_string());
        feed.add_assistant_message();
        feed.complete_turn(3.5);
        match &feed.items[1] {
            FeedItem::AssistantMessage { turn_duration, .. } => {
                assert_eq!(*turn_duration, Some(3.5));
            }
            _ => panic!("Expected AssistantMessage"),
        }
    }

    #[test]
    fn test_has_assistant_in_progress() {
        let mut feed = Feed::new();
        assert!(!feed.has_assistant_in_progress());
        feed.add_user_message("Hello".to_string());
        assert!(!feed.has_assistant_in_progress());
        feed.add_assistant_message();
        assert!(feed.has_assistant_in_progress());
    }

    #[test]
    fn test_clear() {
        let mut feed = Feed::new();
        feed.add_user_message("Hello".to_string());
        feed.add_assistant_message();
        feed.clear();
        assert!(feed.is_empty());
    }

    #[test]
    fn test_dedup_by_id() {
        let mut feed = Feed::new();
        let id = Uuid::new_v4().to_string();
        feed.add_if_new(FeedItem::UserMessage {
            id: id.clone(),
            text: "First".to_string(),
            timestamp: None,
        });
        assert_eq!(feed.items.len(), 1);
        // Try to add same ID again - should be deduped
        feed.add_if_new(FeedItem::UserMessage {
            id: id.clone(),
            text: "Second".to_string(),
            timestamp: None,
        });
        assert_eq!(feed.items.len(), 1);
        // First item's ID is preserved
        assert_eq!(feed.items[0].id(), id);
    }

    #[test]
    fn test_add_thought_no_op_without_assistant() {
        let mut feed = Feed::new();
        feed.add_thought(1.0); // Should not panic
        assert!(feed.is_empty());
    }

    #[test]
    fn test_add_tool_call_no_op_without_assistant() {
        let mut feed = Feed::new();
        feed.add_tool_call("bash".to_string(), "pwd".to_string());
        assert!(feed.is_empty());
    }

    #[test]
    fn test_remove_last_empty_assistant() {
        let mut feed = Feed::new();
        feed.add_user_message("Hello".to_string());
        feed.add_assistant_message();
        feed.remove_last_empty_assistant();
        assert_eq!(feed.items.len(), 1); // Only user message remains
    }

    #[test]
    fn test_remove_last_empty_assistant_keeps_non_empty() {
        let mut feed = Feed::new();
        feed.add_user_message("Hello".to_string());
        feed.add_assistant_message();
        feed.append_to_last("Hi");
        feed.remove_last_empty_assistant();
        assert_eq!(feed.items.len(), 2); // Both remain
    }

    #[test]
    fn test_from_message_items() {
        use crate::components::message_list::MessageItem;
        let messages = vec![
            MessageItem::User { text: "Hello".to_string(), model: None, timestamp: None },
            MessageItem::Assistant { text: "Hi".to_string(), model: None, timestamp: None },
            MessageItem::System { text: "System notice".to_string() },
            // These should be filtered out (now inline in AssistantMessage)
            MessageItem::Thought { duration_secs: 1.0 },
            MessageItem::ToolCall { name: "bash".to_string(), args: "pwd".to_string(), result: None, is_error: false },
        ];
        let feed = Feed::from(messages);
        let items = feed.items();
        assert_eq!(items.len(), 3);
        assert!(matches!(&items[0], FeedItem::UserMessage { text, .. } if text == "Hello"));
        assert!(matches!(&items[1], FeedItem::AssistantMessage { text, .. } if text == "Hi"));
        assert!(matches!(&items[2], FeedItem::SystemNotice { text, .. } if text == "System notice"));
    }

    #[test]
    fn test_assistant_message_with_inline_thoughts_and_tool_calls() {
        use crate::components::message_list::MessageItem;
        let messages = vec![
            MessageItem::User { text: "Hello".to_string(), model: None, timestamp: None },
            MessageItem::Assistant { text: "I'll help you".to_string(), model: None, timestamp: None },
        ];
        let mut feed = Feed::from(messages);
        // Simulate adding thoughts and tool calls inline (as done during streaming)
        feed.add_thought(1.5);
        feed.add_tool_call("bash".to_string(), "ls".to_string());

        let items = feed.items();
        match &items[1] {
            FeedItem::AssistantMessage { thoughts, tool_calls, .. } => {
                assert_eq!(thoughts.len(), 1);
                assert_eq!(thoughts[0].duration, 1.5);
                assert_eq!(tool_calls.len(), 1);
                assert_eq!(tool_calls[0].name, "bash");
            }
            _ => panic!("Expected AssistantMessage"),
        }
    }
}

#[cfg(test)]
mod feed_rendering_tests;
