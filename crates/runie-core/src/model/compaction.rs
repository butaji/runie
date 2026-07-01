//! Session compaction and token accounting.

use crate::message::{now, ChatMessage, MessageMetadata};
use crate::model::state::AppState;
use crate::model::Role;

impl AppState {
    /// Total tokens across all messages.
    pub fn total_tokens(&self) -> usize {
        self.session
            .messages
            .iter()
            .map(|m| {
                self.agent_state()
                    .token_tracker
                    .estimate_input(&m.content())
            })
            .sum()
    }

    /// Compact messages, keeping approximately `keep_recent_tokens` worth of recent content.
    /// Pinned messages are preserved at the beginning. Returns a summary of what was compacted.
    pub fn compact(&mut self, keep_recent_tokens: usize) -> String {
        // Count only non-pinned messages for total
        let non_pinned: Vec<_> = self
            .session
            .messages
            .iter()
            .filter(|m| !m.metadata.pinned)
            .collect();
        let non_pinned_tokens: usize = non_pinned
            .iter()
            .map(|m| {
                self.agent_state()
                    .token_tracker
                    .estimate_input(&m.content())
            })
            .sum();

        if non_pinned_tokens <= keep_recent_tokens {
            return format!(
                "Session has {} non-pinned tokens, no compaction needed",
                non_pinned_tokens
            );
        }

        // Collect pinned messages to preserve
        let pinned: Vec<ChatMessage> = self
            .session
            .messages
            .iter()
            .filter(|m| m.metadata.pinned)
            .cloned()
            .collect();

        // Find how many non-pinned messages to remove
        let mut accumulated = 0usize;
        let mut remove_count = 0usize;
        for msg in non_pinned.iter().rev() {
            accumulated += self
                .agent_state()
                .token_tracker
                .estimate_input(&msg.content());
            remove_count += 1;
            if accumulated >= keep_recent_tokens {
                break;
            }
        }

        // Keep pinned messages + remaining non-pinned + summary
        let total_non_pinned = non_pinned.len();
        let non_pinned_to_keep = total_non_pinned - remove_count;

        // Build new message list: pinned + kept non-pinned + summary
        let kept_non_pinned: Vec<_> = non_pinned.into_iter().take(non_pinned_to_keep).cloned().collect();

        let summary = format!(
            "[Compacted: {} earlier messages removed, keeping ~{} tokens]",
            remove_count, keep_recent_tokens
        );

        let mut new_messages = pinned;
        new_messages.push(ChatMessage {
            role: Role::System,
            timestamp: now(),
            id: "compaction".to_owned(),
            metadata: MessageMetadata {
                compacted: true,
                ..Default::default()
            },
            parts: vec![runie_core::message::Part::Text {
                content: summary.clone(),
            }],
            ..Default::default()
        });
        new_messages.extend(kept_non_pinned);

        self.session_mut().messages = new_messages;
        self.messages_changed();
        summary
    }
}
