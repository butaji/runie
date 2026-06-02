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
            Assistant { text, model: _, timestamp, expanded } => Ok(FeedItem::AssistantMessage {
                id: Uuid::new_v4().to_string(),
                text,
                thoughts: Vec::new(),
                tool_calls: Vec::new(),
                timestamp,
                turn_duration: None,
                thoughts_collapsed: !expanded,
                expanded: true,
            }),
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
