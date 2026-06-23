//! ThinkFilter: event-based middleware that converts inline `<tool_call>` /
//! `</thinking>` / `<thinking>` / `</thinking>` tags in `TextDelta` streams
//! into proper `ThinkingStart` / `ThinkingDelta` / `ThinkingEnd` events.
//!
//! Many providers (DeepSeek, vLLM, OpenRouter, local Ollama models) stream
//! reasoning content as plain text wrapped in these tags inside the same
//! `TextDelta` stream, with no separate `ThinkingDelta` channel. This filter
//! normalizes those inline tags so downstream consumers receive structured
//! reasoning events regardless of provider shape.

use std::mem;

/// Tags that open a thinking block (ASCII, so `str::find` is sufficient).
const OPENING_TAGS: [&str; 2] = ["<tool_call>", "<thinking>"];
/// Tag that closes a thinking block (shared between both opening variants).
const CLOSING_TAG: &str = "</thinking>";

/// Event transformer that buffers partial tags and emits structured thinking
/// events from inline tag-delimited text.
#[derive(Debug, Default)]
pub struct ThinkFilter {
    /// Buffered partial text at the end of a chunk that may be a partial tag.
    /// Cleared on every feed call after being prepended to the new delta.
    buffer: String,
    /// True when the last emitted delta was inside a thinking block.
    in_thinking: bool,
    /// True when we're waiting for a partial opening tag to complete.
    waiting_open_tag: bool,
}

impl ThinkFilter {
    /// Create a new ThinkFilter with empty state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Process a single `LLMEvent` and return zero or more transformed events.
    ///
    /// - `TextDelta` is scanned for opening/closing tags; partial tags are
    ///   buffered until resolved.
    /// - `ThinkingDelta` is passed through unchanged.
    /// - Other event types trigger a buffer flush before passthrough.
    pub fn feed(&mut self, event: runie_core::llm_event::LLMEvent) -> Vec<runie_core::llm_event::LLMEvent> {
        match event {
            runie_core::llm_event::LLMEvent::TextDelta(delta) => self.feed_text(delta),
            runie_core::llm_event::LLMEvent::ThinkingDelta(delta) => {
                vec![runie_core::llm_event::LLMEvent::ThinkingDelta(delta)]
            }
            _ => {
                let mut out = self.flush_buffer();
                out.push(event);
                out
            }
        }
    }

    /// Drain any remaining buffered state and emit final thinking close.
    ///
    /// Call this after the provider stream ends to ensure no content is lost.
    pub fn flush(&mut self) -> Vec<runie_core::llm_event::LLMEvent> {
        self.flush_buffer()
    }

    fn feed_text(&mut self, delta: String) -> Vec<runie_core::llm_event::LLMEvent> {
        if delta.is_empty() {
            return Vec::new();
        }
        let text = prepend_buffer(mem::take(&mut self.buffer), delta);
        let mut out = Vec::new();
        let mut pos = 0;

        while pos < text.len() {
            if self.in_thinking {
                pos = consume_inside(&text, pos, &mut self.in_thinking, &mut self.buffer, &mut out);
            } else {
                pos = consume_outside(&text, pos, &mut self.in_thinking, &mut self.waiting_open_tag, &mut self.buffer, &mut out);
            }
        }

        out
    }

    fn flush_buffer(&mut self) -> Vec<runie_core::llm_event::LLMEvent> {
        let mut out = Vec::new();
        if self.in_thinking {
            if !self.buffer.is_empty() {
                out.push(runie_core::llm_event::LLMEvent::ThinkingDelta(self.buffer.clone()));
            }
            out.push(runie_core::llm_event::LLMEvent::ThinkingEnd {
                id: "inline".to_string(),
            });
        } else if !self.buffer.is_empty() {
            out.push(runie_core::llm_event::LLMEvent::TextDelta(self.buffer.clone()));
        }
        self.buffer.clear();
        self.in_thinking = false;
        out
    }
}

// ============================================================================
// Emit helpers
// ============================================================================

fn emit_text(out: &mut Vec<runie_core::llm_event::LLMEvent>, text: String) {
    if text.is_empty() {
        return;
    }
    out.push(runie_core::llm_event::LLMEvent::TextDelta(text));
}

fn emit_thinking(out: &mut Vec<runie_core::llm_event::LLMEvent>, text: String) {
    if text.is_empty() {
        return;
    }
    out.push(runie_core::llm_event::LLMEvent::ThinkingDelta(text));
}

fn emit_thinking_start(out: &mut Vec<runie_core::llm_event::LLMEvent>) {
    out.push(runie_core::llm_event::LLMEvent::ThinkingStart {
        id: "inline".to_string(),
    });
}

fn emit_thinking_end(out: &mut Vec<runie_core::llm_event::LLMEvent>) {
    out.push(runie_core::llm_event::LLMEvent::ThinkingEnd {
        id: "inline".to_string(),
    });
}

// ============================================================================
// Consume outside thinking
// ============================================================================

/// Handles text outside thinking blocks. Finds opening tags and emits text.
/// Returns the new position after processing.
fn consume_outside(
    text: &str,
    pos: usize,
    in_thinking: &mut bool,
    waiting_open_tag: &mut bool,
    buffer: &mut String,
    out: &mut Vec<runie_core::llm_event::LLMEvent>,
) -> usize {
    let remaining = &text[pos..];
    let next_open = find_next_opening_tag(remaining);

    match next_open {
        None if *waiting_open_tag => {
            // We're waiting for a partial opening tag - buffer the content
            handle_no_tags_outside(remaining, buffer, waiting_open_tag, out);
            text.len()
        }
        None => {
            // Plain text without opening tags - emit but don't buffer
            handle_no_tags_outside(remaining, buffer, waiting_open_tag, out);
            text.len()
        }
        Some((open_tag, open_pos)) => {
            handle_opening_outside(text, remaining, open_tag, open_pos, pos, in_thinking, waiting_open_tag, buffer, out)
        }
    }
}

fn handle_no_tags_outside(
    remaining: &str,
    buffer: &mut String,
    waiting_open_tag: &mut bool,
    out: &mut Vec<runie_core::llm_event::LLMEvent>,
) {
    if let Some(n) = partial_tag_len(remaining) {
        emit_text(out, remaining[..remaining.len() - n].to_string());
        *buffer = remaining[remaining.len() - n..].to_string();
    } else if *waiting_open_tag {
        // We're waiting for a partial opening tag to complete - buffer the content
        emit_text(out, remaining.to_string());
        *buffer = remaining.to_string();
        *waiting_open_tag = false;
    } else {
        // Plain text without tags - emit but don't buffer
        emit_text(out, remaining.to_string());
    }
}

fn handle_opening_outside(
    text: &str,
    remaining: &str,
    open_tag: &str,
    open_pos: usize,
    pos: usize,
    in_thinking: &mut bool,
    waiting_open_tag: &mut bool,
    buffer: &mut String,
    out: &mut Vec<runie_core::llm_event::LLMEvent>,
) -> usize {
    // Emit text before the opening tag.
    if open_pos > 0 {
        emit_text(out, remaining[..open_pos].to_string());
    }
    // Check if opening tag is at end of chunk.
    let after_tag_pos = pos + open_pos + open_tag.len();
    if after_tag_pos >= text.len() {
        // Partial tag at end - buffer it and mark that we're waiting for it.
        *buffer = remaining[open_pos..].to_string();
        *waiting_open_tag = true;
        return pos + remaining.len();
    }
    // Full opening tag followed by content.
    if *in_thinking {
        emit_thinking_end(out);
    }
    emit_thinking_start(out);
    *in_thinking = true;
    *waiting_open_tag = false;
    // Buffer the remaining content after the opening tag
    *buffer = text[after_tag_pos..].to_string();
    after_tag_pos
}

// ============================================================================
// Consume inside thinking
// ============================================================================

/// Handles text inside thinking blocks. Finds closing/nested/opening tags.
/// Returns the new position after processing.
fn consume_inside(
    text: &str,
    pos: usize,
    in_thinking: &mut bool,
    buffer: &mut String,
    out: &mut Vec<runie_core::llm_event::LLMEvent>,
) -> usize {
    let remaining = &text[pos..];
    let next_open = find_next_opening_tag(remaining);
    let next_close = remaining.find(CLOSING_TAG);

    match (next_close, next_open) {
        (None, None) => {
            handle_no_tags_inside(remaining, buffer, out);
            text.len()
        }
        (Some(close_pos), None) => {
            handle_close_only(text, remaining, close_pos, pos, in_thinking, buffer, out)
        }
        (None, Some((open_tag, open_pos))) => {
            handle_nested_opening(text, remaining, open_tag, open_pos, pos, in_thinking, buffer, out)
        }
        (Some(close_pos), Some((open_tag, open_pos))) => {
            if close_pos <= open_pos {
                handle_close_only(text, remaining, close_pos, pos, in_thinking, buffer, out)
            } else {
                handle_nested_opening(text, remaining, open_tag, open_pos, pos, in_thinking, buffer, out)
            }
        }
    }
}

fn handle_no_tags_inside(
    remaining: &str,
    buffer: &mut String,
    out: &mut Vec<runie_core::llm_event::LLMEvent>,
) {
    if let Some(n) = partial_tag_len(remaining) {
        emit_thinking(out, remaining[..remaining.len() - n].to_string());
        *buffer = remaining[remaining.len() - n..].to_string();
    } else {
        emit_thinking(out, remaining.to_string());
        // Only buffer partial tags, not thinking content
    }
}

fn handle_close_only(
    text: &str,
    remaining: &str,
    close_pos: usize,
    pos: usize,
    in_thinking: &mut bool,
    buffer: &mut String,
    out: &mut Vec<runie_core::llm_event::LLMEvent>,
) -> usize {
    emit_thinking(out, remaining[..close_pos].to_string());
    let new_pos = pos + close_pos + CLOSING_TAG.len();
    // Check for second </thinking> (possibly separated by whitespace).
    let mut after_first = new_pos;
    while after_first < text.len() && text[after_first..].starts_with(' ') {
        after_first += 1;
    }
    if *in_thinking && after_first < text.len() && text[after_first..].starts_with(CLOSING_TAG) {
        emit_thinking_end(out);
        *in_thinking = false;
        emit_text(out, text[new_pos..].to_string());
        buffer.clear();
        return text.len();
    }
    // Normal close.
    emit_thinking_end(out);
    *in_thinking = false;
    new_pos
}

fn handle_nested_opening(
    text: &str,
    remaining: &str,
    open_tag: &str,
    open_pos: usize,
    pos: usize,
    in_thinking: &mut bool,
    buffer: &mut String,
    out: &mut Vec<runie_core::llm_event::LLMEvent>,
) -> usize {
    emit_thinking(out, remaining[..open_pos].to_string());
    emit_thinking_end(out);
    *in_thinking = false;
    let after_tag_pos = pos + open_pos + open_tag.len();
    if after_tag_pos >= remaining.len() {
        // Partial tag at end - buffer it.
        *buffer = open_tag.to_string();
        return text.len();
    }
    emit_thinking_start(out);
    *in_thinking = true;
    after_tag_pos
}

// ============================================================================
// Private helpers
// ============================================================================

/// Find the earliest opening tag in the given text slice.
///
/// Returns the tag string and its byte offset within the slice.
fn find_next_opening_tag(text: &str) -> Option<(&str, usize)> {
    let mut best_pos = None;
    let mut best_tag = None;
    for tag in &OPENING_TAGS {
        if let Some(pos) = text.find(tag) {
            match best_pos {
                None => {
                    best_pos = Some(pos);
                    best_tag = Some(*tag);
                }
                Some(bp) if pos < bp => {
                    best_pos = Some(pos);
                    best_tag = Some(*tag);
                }
                _ => {}
            }
        }
    }
    best_tag.map(|tag| (tag, best_pos.unwrap()))
}

/// Returns `Some(n)` if the end of `text` is a partial closing tag (`n` =
/// number of bytes that match the prefix of `CLOSING_TAG`). Returns `None` if
/// the text ends with a complete tag or has no partial match.
fn partial_tag_len(text: &str) -> Option<usize> {
    if text.len() < CLOSING_TAG.len() && CLOSING_TAG.starts_with(text) {
        return Some(text.len());
    }
    for n in 1..CLOSING_TAG.len() {
        if text.len() >= n && CLOSING_TAG.starts_with(&text[text.len() - n..]) {
            return Some(n);
        }
    }
    None
}

/// Prepend any buffered content to the new delta.
fn prepend_buffer(buffer: String, delta: String) -> String {
    if buffer.is_empty() {
        delta
    } else {
        format!("{}{}", buffer, delta)
    }
}

#[cfg(test)]
mod inner_tests {
    use super::*;

    #[test]
    fn prepend_buffer_empty() {
        assert_eq!(prepend_buffer(String::new(), "hello".into()), "hello");
    }

    #[test]
    fn prepend_buffer_combines() {
        assert_eq!(prepend_buffer("<tool".into(), "_call>hi".into()), "<tool_call>hi");
    }
}
