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
        let non_pinned: Vec<_> = self
            .session
            .messages
            .iter()
            .filter(|m| !m.metadata.pinned)
            .collect();
        let non_pinned_tokens = self.sum_tokens(&non_pinned);

        if non_pinned_tokens <= keep_recent_tokens {
            return format!(
                "Session has {} non-pinned tokens, no compaction needed",
                non_pinned_tokens
            );
        }

        let pinned: Vec<ChatMessage> = self
            .session
            .messages
            .iter()
            .filter(|m| m.metadata.pinned)
            .cloned()
            .collect();

        let remove_count = self.count_messages_to_remove(&non_pinned, keep_recent_tokens);
        let non_pinned_to_keep = non_pinned.len().saturating_sub(remove_count);
        let kept_non_pinned: Vec<_> = non_pinned
            .into_iter()
            .take(non_pinned_to_keep)
            .cloned()
            .collect();

        let summary = format!(
            "[Compacted: {} earlier messages removed, keeping ~{} tokens]",
            remove_count, keep_recent_tokens
        );

        let new_messages =
            Self::build_compacted_messages(pinned, kept_non_pinned, summary.clone());

        self.session_mut().messages = new_messages;
        self.messages_changed();
        summary
    }

    /// Sum tokens for a slice of messages.
    fn sum_tokens(&self, messages: &[&crate::message::ChatMessage]) -> usize {
        messages
            .iter()
            .map(|m| self.agent_state().token_tracker.estimate_input(&m.content()))
            .sum()
    }

    /// Count how many non-pinned messages to remove to fit within token limit.
    fn count_messages_to_remove(
        &self,
        non_pinned: &[&crate::message::ChatMessage],
        keep_recent_tokens: usize,
    ) -> usize {
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
        remove_count
    }

    /// Build the final message list: pinned + summary + kept non-pinned.
    fn build_compacted_messages(
        pinned: Vec<ChatMessage>,
        kept_non_pinned: Vec<ChatMessage>,
        summary: String,
    ) -> Vec<ChatMessage> {
        let mut new_messages = pinned;
        new_messages.push(ChatMessage {
            role: Role::System,
            timestamp: now(),
            id: "compaction".to_owned(),
            metadata: MessageMetadata {
                compacted: true,
                ..Default::default()
            },
            parts: vec![runie_core::message::Part::Text { content: summary }],
            ..Default::default()
        });
        new_messages.extend(kept_non_pinned);
        new_messages
    }
}
