//! Session compaction and token accounting.

use std::sync::LazyLock;

use regex::Regex;

use crate::message::{now, ChatMessage, MessageMetadata, MessageOrigin, Part};
use crate::model::state::AppState;
use crate::model::Role;

// Lazy-initialized regex patterns (compiled once at first use).
// The patterns are hardcoded and syntactically valid — unwrap documents this invariant.
static FENCE_PAT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(```[a-zA-Z0-9_-]*)\n([\s\S]*?)\n```").unwrap());
static DETAILS_PAT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)<details>\s*\n?([\s\S]*?)</details>").unwrap());

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

    /// Truncate long code blocks and `<details>` sections within messages
    /// before dropping whole messages. Tool-call/tool-result pairs are kept atomic.
    ///
    /// Returns a summary of what was truncated, or an empty string if no truncation happened.
    pub fn truncate_messages_structurally(&mut self) -> String {
        let mut total_truncated = 0usize;
        let mut truncated_parts = 0usize;

        for msg in self.session_mut().messages.iter_mut() {
            if msg.metadata.pinned {
                continue;
            }
            for part in msg.parts.iter_mut() {
                if let Part::Text { content } = part {
                    let (truncated_count, did_truncate) = truncate_structural(content, TRUNCATE_KEEP_LINES);
                    if did_truncate {
                        total_truncated += truncated_count;
                        truncated_parts += 1;
                    }
                }
            }
        }

        if total_truncated > 0 {
            self.messages_changed();
            let summary = format!(
                "[Truncated {} lines across {} code blocks / details sections]",
                total_truncated, truncated_parts
            );
            // Insert a summary message.
            self.session_mut()
                .messages
                .push(ChatMessage::system(summary.clone()));
            self.messages_changed();
            summary
        } else {
            String::new()
        }
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

        let new_messages = Self::build_compacted_messages(pinned, kept_non_pinned, summary.clone());

        self.session_mut().messages = new_messages;
        self.messages_changed();
        summary
    }

    /// Sum tokens for a slice of messages.
    fn sum_tokens(&self, messages: &[&crate::message::ChatMessage]) -> usize {
        messages
            .iter()
            .map(|m| {
                self.agent_state()
                    .token_tracker
                    .estimate_input(&m.content())
            })
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
            metadata: MessageMetadata { compacted: true, origin: MessageOrigin::Compaction, ..Default::default() },
            parts: vec![runie_core::message::Part::Text { content: summary }],
            ..Default::default()
        });
        new_messages.extend(kept_non_pinned);
        new_messages
    }
}

/// Number of first/last lines to keep when truncating a code block or details section.
const TRUNCATE_KEEP_LINES: usize = 5;

/// Truncate long code blocks and `<details>` sections in a markdown string,
/// keeping the first and last `keep` lines of each block with a `[...]` placeholder.
///
/// Returns `(lines_removed, was_modified)`.
fn truncate_structural(content: &mut String, keep: usize) -> (usize, bool) {
    let mut removed = 0usize;
    let mut modified = false;

    // Truncate fenced code blocks first (they may appear inside details).
    let (block_removed, block_modified) = truncate_fenced_code_blocks(content, keep);
    removed += block_removed;
    modified = modified || block_modified;

    // Then truncate <details> sections.
    let (details_removed, details_modified) = truncate_details_blocks(content, keep);
    removed += details_removed;
    modified = modified || details_modified;

    (removed, modified)
}

/// Truncate fenced code blocks (```...```) in a markdown string,
/// keeping first and last `keep` lines with a `[...]` placeholder.
fn truncate_fenced_code_blocks(content: &mut String, keep: usize) -> (usize, bool) {
    let mut removed = 0usize;
    let mut modified = false;

    // We need to iterate carefully since we modify the string.
    // Strategy: find all matches, compute truncated version, then rebuild.
    let mut result = String::with_capacity(content.len());
    let mut last_end = 0;

    for cap in FENCE_PAT.captures_iter(content) {
        // `captures_iter` always yields at least the full match at index 0.
        let full = cap.get(0).unwrap();
        let lang = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let inner = cap.get(2).map(|m| m.as_str()).unwrap_or("");

        result.push_str(&content[last_end..full.start()]);

        let lines: Vec<&str> = inner.lines().collect();
        if lines.len() > keep * 2 + 1 {
            let first: String = lines[..keep].join("\n");
            let last: String = lines[lines.len() - keep..].join("\n");
            let middle_lines = lines.len() - keep * 2;
            result.push_str(lang);
            result.push('\n');
            result.push_str(&first);
            result.push_str("\n\n[...] // ");
            result.push_str(&format!("{} intermediate lines removed\n\n", middle_lines));
            result.push_str(&last);
            result.push_str("\n```");
            removed += middle_lines;
            modified = true;
        } else {
            result.push_str(full.as_str());
        }

        last_end = full.end();
    }

    result.push_str(&content[last_end..]);
    if modified {
        *content = result;
    }

    (removed, modified)
}

/// Truncate `<details>` sections in an HTML/markdown string,
/// keeping first and last `keep` lines of the body with a `[...]` placeholder.
fn truncate_details_blocks(content: &mut String, keep: usize) -> (usize, bool) {
    let mut removed = 0usize;
    let mut modified = false;

    let mut result = String::with_capacity(content.len());
    let mut last_end = 0;

    for cap in DETAILS_PAT.captures_iter(content) {
        // `captures_iter` always yields at least the full match at index 0.
        let full = cap.get(0).unwrap();
        let inner = cap.get(1).map(|m| m.as_str()).unwrap_or("");

        result.push_str(&content[last_end..full.start()]);

        let lines: Vec<&str> = inner.lines().collect();
        if lines.len() > keep * 2 + 1 {
            let first: String = lines[..keep].join("\n");
            let last: String = lines[lines.len() - keep..].join("\n");
            let middle_lines = lines.len() - keep * 2;
            result.push_str("<details>\n");
            result.push_str(&first);
            result.push_str("\n\n[...] // ");
            result.push_str(&format!("{} intermediate lines removed\n\n", middle_lines));
            result.push_str(&last);
            result.push_str("\n</details>");
            removed += middle_lines;
            modified = true;
        } else {
            result.push_str(full.as_str());
        }

        last_end = full.end();
    }

    result.push_str(&content[last_end..]);
    if modified {
        *content = result;
    }

    (removed, modified)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_structural_code_block_short() {
        let mut s = "hello world".to_string();
        let (removed, modified) = truncate_structural(&mut s, 5);
        assert!(!modified);
        assert_eq!(s, "hello world");
        assert_eq!(removed, 0);
    }

    #[test]
    fn truncate_structural_code_block_long() {
        let mut s = "```\nline1\nline2\nline3\nline4\nline5\nline6\nline7\nline8\nline9\nline10\nline11\nline12\n```"
            .to_string();
        let (removed, modified) = truncate_structural(&mut s, 2);
        assert!(modified);
        assert!(removed > 0);
        assert!(s.contains("[...]"));
        assert!(s.contains("line1"));
        assert!(s.contains("line11"));
        assert!(s.contains("line12"));
        // Middle lines should be gone
        assert!(!s.contains("line5"));
    }

    #[test]
    fn truncate_structural_details_long() {
        let mut s = "<details>\nline1\nline2\nline3\nline4\nline5\nline6\nline7\n</details>".to_string();
        let (removed, modified) = truncate_structural(&mut s, 1);
        assert!(modified);
        assert!(removed > 0);
        assert!(s.contains("[...]"));
        assert!(s.contains("line1"));
        assert!(s.contains("line7"));
        assert!(!s.contains("line3"));
    }

    #[test]
    fn truncate_structural_code_block_actually_truncates() {
        // Verify that a code block long enough IS truncated.
        let mut s = "```\nline1\nline2\nline3\nline4\nline5\nline6\nline7\nline8\nline9\nline10\n```".to_string();
        let (removed, modified) = truncate_structural(&mut s, 2);
        assert!(modified);
        assert!(removed > 0);
        assert!(s.contains("[...]"));
        assert!(s.contains("line1"));
        assert!(s.contains("line10"));
        // Middle lines removed
        assert!(!s.contains("line4"));
    }

    #[test]
    fn truncate_structural_no_fence_no_change() {
        let mut s = "This is plain text without any code blocks or details".to_string();
        let (removed, modified) = truncate_structural(&mut s, 5);
        assert!(!modified);
        assert_eq!(removed, 0);
        assert_eq!(s, "This is plain text without any code blocks or details");
    }
}
