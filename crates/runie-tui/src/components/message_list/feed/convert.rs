//! Conversion from MessageItem to FeedItem for rendering pipeline migration.
//!
//! Note: Thought and ToolCall items are filtered out (now inline in AssistantMessage).
//! Separator items are preserved for turn timing display.

use super::{Feed, FeedItem};
use crate::components::message_list::MessageItem;
use uuid::Uuid;

impl TryFrom<MessageItem> for FeedItem {
    type Error = ();

    fn try_from(item: MessageItem) -> Result<Self, Self::Error> {
        use MessageItem::*;
        match item {
            User { text, timestamp, .. } => Ok(FeedItem::UserMessage {
                id: Uuid::new_v4().to_string(),
                text,
                timestamp,
            }),
            Assistant { text, model: _, timestamp, expanded: _, thought_duration, turn_duration } => {
                // Convert thought_duration to a Thought struct for the thoughts vec
                let thoughts = thought_duration
                    .map(|d| super::Thought { duration: d })
                    .into_iter()
                    .collect();
                Ok(FeedItem::AssistantMessage {
                    id: Uuid::new_v4().to_string(),
                    text,
                    thoughts,
                    tool_calls: Vec::new(),
                    timestamp,
                    turn_duration,
                    thoughts_collapsed: false, // Always false during streaming - ensures think blocks render
                    expanded: true,
                    streaming_thinking_elapsed_ms: None,
                    streaming_total_elapsed_ms: None,
                    streaming_download_bytes: None,
                })
            }
            System { text } => Ok(FeedItem::SystemNotice { text }),
            Error { message, .. } => Ok(FeedItem::SystemNotice { text: format!("Error: {}", message) }),
            Separator { elapsed_secs, tool_calls, tokens_used } =>
                Ok(FeedItem::Separator { elapsed_secs, tool_calls, tokens_used }),
            ToolRunning { name, args, duration_ms, total_elapsed_ms, download_bytes } =>
                Ok(FeedItem::ToolRunning { name, args, duration_ms, total_elapsed_ms, download_bytes }),
            ToolComplete { name, result, lines } =>
                Ok(FeedItem::ToolComplete { name, result, lines }),
            Thought { .. } | ToolCall { .. } | Edit { .. }
            | PlanStep { .. } | Interrupt | Rewind { .. } => Err(()),
        }
    }
}

impl From<Vec<MessageItem>> for Feed {
    fn from(messages: Vec<MessageItem>) -> Self {
        let mut feed = Feed::new();
        for item in messages {
            if let Ok(feed_item) = FeedItem::try_from(item) {
                feed.items.push(feed_item);
            }
        }
        feed
    }
}

/// Borrowed conversion: clones nothing. Used on the hot path when the
/// caller doesn't need to mutate or strip from the source items.
impl From<&[MessageItem]> for Feed {
    fn from(messages: &[MessageItem]) -> Self {
        let mut feed = Feed::new();
        for item in messages {
            if let Ok(feed_item) = FeedItem::try_from(item.clone()) {
                feed.items.push(feed_item);
            }
        }
        feed
    }
}
