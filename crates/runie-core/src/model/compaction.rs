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
            .map(|m| self.agent.token_tracker.estimate_input(&m.content))
            .sum()
    }

    /// Compact messages, keeping approximately `keep_recent_tokens` worth of recent content.
    /// Pinned messages are preserved. Returns a summary of what was compacted.
    pub fn compact(&mut self, keep_recent_tokens: usize) -> String {
        let total = self.total_tokens();
        if total <= keep_recent_tokens {
            return format!("Session has {} tokens, no compaction needed", total);
        }

        let cut_idx = self.find_compact_cut_index(keep_recent_tokens);
        if cut_idx == 0 {
            return "Cannot compact: all messages are recent".to_string();
        }

        let removed_count = cut_idx;
        self.session.messages.drain(..cut_idx);
        let summary = format!(
            "[Compacted: {} earlier messages removed, keeping ~{} tokens]",
            removed_count, keep_recent_tokens
        );
        self.session.messages.insert(
            0,
            ChatMessage {
                role: Role::System,
                content: summary.clone(),
                timestamp: now(),
                id: "compaction".to_string(),
                metadata: MessageMetadata {
                    compacted: true,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        self.messages_changed();
        summary
    }

    fn find_compact_cut_index(&self, keep_recent_tokens: usize) -> usize {
        let mut accumulated = 0usize;
        let mut cut_idx = 0usize;
        for (i, msg) in self.session.messages.iter().enumerate().rev() {
            // Skip pinned messages
            if msg.metadata.pinned {
                continue;
            }
            accumulated += self.agent.token_tracker.estimate_input(&msg.content);
            if accumulated >= keep_recent_tokens {
                cut_idx = i;
                break;
            }
        }
        while cut_idx < self.session.messages.len() {
            match self.session.messages[cut_idx].role {
                Role::User | Role::Assistant => break,
                _ => cut_idx += 1,
            }
        }
        cut_idx
    }
}
