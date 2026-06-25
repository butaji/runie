//! ThinkFilter: converts inline `<tool_call>` / `<thinking>` / `</thinking>` /
//! `</tool_call>` tags in `TextDelta` streams into structured thinking events.

use runie_core::provider_event::ProviderEvent;
use std::mem;

/// Tags that open a thinking block.
const OPENING_TAGS: [&str; 2] = ["<tool_call>", "<thinking>"];
const CLOSING_THINKING: &str = "</thinking>";
const CLOSING_TOOL_CALL: &str = "</tool_call>";

/// State machine for tracking position within a thinking block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum ThinkState {
    /// Normal text outside any thinking block.
    #[default]
    Outside,
    /// Just exited a thinking block; next `</thinking>` should be skipped.
    PostThought,
    /// Inside a thinking block, emitting ThinkingDelta.
    Inside,
    /// Buffered a partial opening tag; waiting for the rest.
    WaitingPartialOpen,
}

/// Event transformer that buffers partial tags and emits structured thinking events.
#[derive(Debug, Default)]
pub struct ThinkFilter {
    buffer: String,
    state: ThinkState,
}

impl ThinkFilter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Process a single `ProviderEvent`. Returns zero or more transformed events.
    pub fn feed(&mut self, event: ProviderEvent) -> Vec<ProviderEvent> {
        match event {
            ProviderEvent::TextDelta(delta) => self.feed_text(delta),
            ProviderEvent::ThinkingDelta(delta) => vec![ProviderEvent::ThinkingDelta(delta)],
            other => {
                let mut out = self.flush_buffer();
                out.push(other);
                out
            }
        }
    }

    /// Drain buffered state and emit final thinking close.
    pub fn flush(&mut self) -> Vec<ProviderEvent> {
        self.flush_buffer()
    }

    fn feed_text(&mut self, delta: String) -> Vec<ProviderEvent> {
        if delta.is_empty() {
            return Vec::new();
        }
        let text = prepend_buffer(mem::take(&mut self.buffer), delta);
        let mut out = Vec::new();
        let mut pos = 0usize;
        while pos < text.len() {
            match self.state {
                ThinkState::Outside => pos = self.consume_outside(&text, pos, &mut out),
                ThinkState::PostThought => pos = self.consume_post_thought(&text, pos, &mut out),
                ThinkState::Inside => pos = self.consume_inside(&text, pos, &mut out),
                ThinkState::WaitingPartialOpen => {
                    self.state = ThinkState::Outside;
                    pos = self.consume_outside(&text, pos, &mut out);
                }
            }
        }
        out
    }

    fn flush_buffer(&mut self) -> Vec<ProviderEvent> {
        let mut out = Vec::new();
        match self.state {
            ThinkState::Inside => {
                if !self.buffer.is_empty() {
                    out.push(ProviderEvent::ThinkingDelta(self.buffer.clone()));
                }
                out.push(ProviderEvent::ThinkingEnd {
                    id: "inline".into(),
                });
            }
            ThinkState::WaitingPartialOpen => {
                emit_thinking_start(&mut out);
                if !self.buffer.is_empty() {
                    out.push(ProviderEvent::ThinkingDelta(self.buffer.clone()));
                }
                out.push(ProviderEvent::ThinkingEnd {
                    id: "inline".into(),
                });
            }
            _ if !self.buffer.is_empty() => {
                out.push(ProviderEvent::TextDelta(self.buffer.clone()));
            }
            _ => {}
        }
        self.buffer.clear();
        self.state = ThinkState::Outside;
        out
    }

    // =========================================================================
    // Post-thought: skip the duplicate </thinking> then return to outside
    // =========================================================================

    fn consume_post_thought(
        &mut self,
        text: &str,
        pos: usize,
        out: &mut Vec<ProviderEvent>,
    ) -> usize {
        self.state = ThinkState::Outside;
        let remaining = &text[pos..];
        let ws_len = remaining.chars().take_while(|c| c.is_whitespace()).count();
        let after_ws = pos + ws_len;
        let after_ws_text = &text[after_ws..];
        if after_ws_text.is_empty() {
            return text.len();
        }
        let tag_len = if after_ws_text.starts_with(CLOSING_THINKING) {
            CLOSING_THINKING.len()
        } else if after_ws_text.starts_with(CLOSING_TOOL_CALL) {
            CLOSING_TOOL_CALL.len()
        } else {
            return self.consume_outside(text, after_ws, out);
        };
        after_ws + tag_len
    }

    // =========================================================================
    // Outside: find text or opening tag
    // =========================================================================

    fn consume_outside(&mut self, text: &str, pos: usize, out: &mut Vec<ProviderEvent>) -> usize {
        let remaining = &text[pos..];
        // Check for partial opening tag at start.
        if starts_with_opening_tag(remaining).is_none()
            && starts_partial_opening_tag(remaining).is_some() {
                self.buffer = remaining.to_owned();
                self.state = ThinkState::WaitingPartialOpen;
                return text.len();
            }
        let next_open = find_next_opening_tag(remaining);
        match next_open {
            None => {
                if matches!(
                    self.state,
                    ThinkState::WaitingPartialOpen | ThinkState::Inside
                ) {
                    self.buffer = remaining.to_owned();
                } else {
                    emit_text(out, remaining.to_owned());
                }
                text.len()
            }
            Some((open_tag, open_pos)) => {
                if open_pos > 0 {
                    emit_text(out, remaining[..open_pos].to_string());
                }
                let after_tag_pos = pos + open_pos + open_tag.len();
                if after_tag_pos >= text.len() {
                    self.buffer = remaining[open_pos..].to_string();
                    self.state = ThinkState::WaitingPartialOpen;
                    return text.len();
                }
                emit_thinking_start(out);
                self.state = ThinkState::Inside;
                self.buffer = text[after_tag_pos..].to_string();
                after_tag_pos
            }
        }
    }

    // =========================================================================
    // Inside: find closing tag or nested opening tag
    // =========================================================================

    fn consume_inside(&mut self, text: &str, pos: usize, out: &mut Vec<ProviderEvent>) -> usize {
        let remaining = &text[pos..];
        let next_open = find_next_opening_tag(remaining);
        let close_pos = find_earliest_close(remaining);
        match close_pos {
            None => {
                emit_thinking(out, remaining.to_owned());
                text.len()
            }
            Some((cp, ct)) => self.handle_close_before_open(remaining, pos, next_open, cp, ct, out),
        }
    }

    fn handle_close_before_open(
        &mut self,
        remaining: &str,
        pos: usize,
        next_open: Option<(&str, usize)>,
        close_pos: usize,
        close_tag: &str,
        out: &mut Vec<ProviderEvent>,
    ) -> usize {
        match next_open {
            None => self.emit_close_and_end(remaining, pos, close_pos, close_tag, out),
            Some((_open_tag, open_pos)) if close_pos <= open_pos => {
                self.emit_close_and_end(remaining, pos, close_pos, close_tag, out)
            }
            Some((open_tag, open_pos)) => {
                emit_thinking(out, remaining[..open_pos].to_string());
                emit_thinking_end(out);
                self.state = ThinkState::PostThought;
                let after_tag_pos = pos + open_pos + open_tag.len();
                if after_tag_pos >= remaining.len() {
                    self.buffer = open_tag.to_owned();
                    self.state = ThinkState::WaitingPartialOpen;
                    return remaining.len();
                }
                emit_thinking_start(out);
                self.state = ThinkState::Inside;
                self.buffer.clear();
                after_tag_pos
            }
        }
    }

    fn emit_close_and_end(
        &mut self,
        remaining: &str,
        pos: usize,
        close_pos: usize,
        close_tag: &str,
        out: &mut Vec<ProviderEvent>,
    ) -> usize {
        emit_thinking(out, remaining[..close_pos].to_string());
        let new_pos = pos + close_pos + close_tag.len();
        emit_thinking_end(out);
        self.transition_to_post_thought();
        new_pos
    }

    fn transition_to_post_thought(&mut self) {
        self.buffer.clear();
        self.state = ThinkState::PostThought;
    }
}

// ============================================================================
// Emit helpers
// ============================================================================

fn emit_text(out: &mut Vec<ProviderEvent>, text: String) {
    if !text.is_empty() {
        out.push(ProviderEvent::TextDelta(text));
    }
}

fn emit_thinking(out: &mut Vec<ProviderEvent>, text: String) {
    if !text.is_empty() {
        out.push(ProviderEvent::ThinkingDelta(text));
    }
}

fn emit_thinking_start(out: &mut Vec<ProviderEvent>) {
    out.push(ProviderEvent::ThinkingStart {
        id: "inline".into(),
    });
}

fn emit_thinking_end(out: &mut Vec<ProviderEvent>) {
    out.push(ProviderEvent::ThinkingEnd {
        id: "inline".into(),
    });
}

// ============================================================================
// Private helpers
// ============================================================================

/// Returns Some(tag) if `text` starts with a complete opening tag.
fn starts_with_opening_tag(text: &str) -> Option<&'static str> {
    for tag in &OPENING_TAGS {
        if text.starts_with(tag) {
            return Some(*tag);
        }
    }
    None
}

/// Returns Some(tag) if the start of `text` is a partial opening tag.
fn starts_partial_opening_tag(text: &str) -> Option<&'static str> {
    for tag in &OPENING_TAGS {
        if tag.starts_with(text) {
            return Some(*tag);
        }
    }
    None
}

/// Find the earliest opening tag in the given text slice.
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

/// Find the earliest closing tag (</thinking> or </tool_call>).
fn find_earliest_close(text: &str) -> Option<(usize, &'static str)> {
    let close_t = text.find(CLOSING_THINKING);
    let close_tc = text.find(CLOSING_TOOL_CALL);
    match (close_t, close_tc) {
        (Some(a), Some(b)) if a <= b => Some((a, CLOSING_THINKING)),
        (Some(a), None) => Some((a, CLOSING_THINKING)),
        (None, Some(b)) => Some((b, CLOSING_TOOL_CALL)),
        (Some(_), Some(b)) => Some((b, CLOSING_TOOL_CALL)),
        (None, None) => None,
    }
}

/// Prepend any buffered content to the new delta.
pub(crate) fn prepend_buffer(buffer: String, delta: String) -> String {
    if buffer.is_empty() {
        delta
    } else {
        format!("{}{}", buffer, delta)
    }
}

#[cfg(test)]
mod tests;
