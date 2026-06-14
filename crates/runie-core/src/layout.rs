//! Layout helpers shared between core and the TUI renderer.
//!
//! These helpers are intentionally free of Ratatui types so that core can
//! use them for scroll math while the TUI uses the same logic to produce
//! actual rendered lines.

use crate::display_width;
use crate::markdown::{extract_code_blocks, CodeBlock};
use crate::ui::elements::Element;

/// User message prefix glyph (must match `runie_tui::theme::GLYPH_USER`).
pub const GLYPH_USER: &str = "❯ ";
/// Agent message prefix glyph (must match `runie_tui::theme::GLYPH_AGENT`).
pub const GLYPH_AGENT: &str = "→ ";
/// Indented continuation glyph (must match `runie_tui::theme::GLYPH_INDENT`).
pub const GLYPH_INDENT: &str = "  ";

/// Number of terminal rows an element renders to at the given viewport
/// width. This uses the same wrapping rules as `runie_tui::ui::messages::to_lines`,
/// so scroll math in core stays consistent with the actual Ratatui output.
pub fn element_line_count(element: &Element, width: u16) -> usize {
    if width == 0 {
        return fallback_line_count(element);
    }

    match element {
        Element::Spacer { .. } => 1,
        Element::UserMessage { content, timestamp } => {
            user_message_line_count(content, *timestamp, width)
        }
        Element::AgentMessage {
            content, timestamp, ..
        } => agent_message_line_count(content, *timestamp, width),
        Element::Thinking { .. } => 1,
        Element::ThoughtMarker { content, .. } => thought_marker_line_count(content, width),
        Element::ThoughtSummary { .. } => 1,
        Element::ToolRunning { .. } => 1,
        Element::ToolDone { output, .. } => tool_done_line_count(output),
        Element::ToolSummary { .. } => 1,
        Element::TurnComplete { .. } => 1,
    }
}

fn fallback_line_count(element: &Element) -> usize {
    match element {
        Element::Spacer { .. } => 1,
        Element::UserMessage { content, .. } => content.lines().count().max(1) + 2,
        Element::AgentMessage { content, .. } => content.lines().count().max(1),
        Element::Thinking { .. } => 1,
        Element::ThoughtMarker { content, .. } => content.lines().count().max(1),
        Element::ThoughtSummary { .. } => 1,
        Element::ToolRunning { .. } => 1,
        Element::ToolDone { output, .. } => {
            if output.is_empty() {
                1
            } else {
                1 + output.lines().count()
            }
        }
        Element::ToolSummary { .. } => 1,
        Element::TurnComplete { .. } => 1,
    }
}

fn user_message_line_count(content: &str, timestamp: f64, width: u16) -> usize {
    let inner_width = width.saturating_sub(2);
    if inner_width == 0 {
        return content.lines().count().max(1) + 2;
    }

    let prefix_width = glyph_width(GLYPH_USER);
    let indent_width = glyph_width(GLYPH_INDENT);
    let ts_str = crate::labels::format_timestamp(timestamp);
    let ts_width = ts_str.len() as u16 + 1;

    let first_w = inner_width
        .saturating_sub(prefix_width)
        .saturating_sub(ts_width);
    let rest_w = inner_width.saturating_sub(indent_width);

    let explicit_lines: Vec<&str> = content.lines().collect();
    let mut content_lines = 0usize;
    for (i, line) in explicit_lines.iter().enumerate() {
        let w = if i == 0 { first_w } else { rest_w };
        content_lines += word_wrap(line, w, rest_w).len().max(1);
    }

    // Top and bottom margin lines, plus at least one content line.
    2 + content_lines.max(1)
}

fn agent_message_line_count(content: &str, timestamp: f64, width: u16) -> usize {
    let inner_width = width.saturating_sub(2);
    if inner_width == 0 {
        return content.lines().count().max(1);
    }

    let prefix_width = glyph_width(GLYPH_AGENT);
    let indent_width = glyph_width(GLYPH_INDENT);
    let ts_str = crate::labels::format_timestamp(timestamp);
    let ts_width = ts_str.len() as u16 + 1;

    let mut total = 0usize;
    let mut is_first = true;
    for block in extract_code_blocks(content) {
        total += markdown_block_line_count(
            &block,
            inner_width,
            prefix_width,
            indent_width,
            ts_width,
            &mut is_first,
        );
    }
    total.max(1)
}

fn markdown_block_line_count(
    block: &CodeBlock,
    inner_width: u16,
    prefix_width: u16,
    indent_width: u16,
    ts_width: u16,
    is_first: &mut bool,
) -> usize {
    let lines = match block {
        CodeBlock::Text(text) => text_block_line_count(
            text,
            inner_width,
            prefix_width,
            indent_width,
            ts_width,
            *is_first,
        ),
        CodeBlock::Code { content, .. } => 1 + content.lines().count(),
        CodeBlock::List { items, .. } => items.len(),
        CodeBlock::Blockquote(text) => text.lines().count().max(1),
    };
    *is_first = false;
    lines
}

fn text_block_line_count(
    text: &str,
    inner_width: u16,
    prefix_width: u16,
    indent_width: u16,
    ts_width: u16,
    is_first: bool,
) -> usize {
    let first_w = if is_first {
        inner_width
            .saturating_sub(prefix_width)
            .saturating_sub(ts_width)
    } else {
        inner_width.saturating_sub(indent_width)
    };
    let rest_w = inner_width.saturating_sub(indent_width);
    let mut lines = 0usize;
    for (i, line) in text.lines().enumerate() {
        let w = if i == 0 { first_w } else { rest_w };
        lines += word_wrap(line, w, rest_w).len().max(1);
    }
    lines
}

fn thought_marker_line_count(content: &str, width: u16) -> usize {
    let inner_width = width.saturating_sub(2);
    if inner_width == 0 {
        return content.lines().count().max(1);
    }

    let mut lines = 0usize;
    for line in content.lines() {
        if line.is_empty() {
            lines += 1;
        } else {
            lines += word_wrap(line, inner_width, inner_width).len().max(1);
        }
    }
    lines.max(1)
}

fn tool_done_line_count(output: &str) -> usize {
    if output.is_empty() {
        1
    } else {
        1 + output.lines().count()
    }
}

fn glyph_width(s: &str) -> u16 {
    display_width::width(s)
}

/// Word-wrap `text` into lines using display-cell width so wide characters
/// (CJK, emoji) count as two cells and are never split.
pub fn word_wrap(text: &str, first_width: u16, _rest_width: u16) -> Vec<String> {
    let mut state = WrapState::new(first_width);
    for word in text.split_whitespace() {
        state.consume_word(word);
    }
    state.finish(text)
}

struct WrapState {
    result: Vec<String>,
    current: String,
    width: u16,
    max: u16,
}

impl WrapState {
    fn new(first_width: u16) -> Self {
        Self {
            result: Vec::new(),
            current: String::new(),
            width: 0,
            max: first_width.max(1),
        }
    }

    fn consume_word(&mut self, word: &str) {
        let w = display_width::width(word);
        let need_space = !self.current.is_empty();
        if need_space && self.width + 1 + w > self.max {
            self.flush();
        }
        if self.current.is_empty() && w > self.max {
            self.split_word(word);
            return;
        }
        if need_space && !self.current.is_empty() {
            self.current.push(' ');
            self.width += 1;
        }
        self.current.push_str(word);
        self.width += w;
    }

    fn split_word(&mut self, word: &str) {
        let mut remaining = word;
        while !remaining.is_empty() {
            let (chunk, rest) = display_width::split_at_width(remaining, self.max);
            if chunk.is_empty() {
                let mut chars = remaining.chars();
                if let Some(ch) = chars.next() {
                    self.result.push(ch.to_string());
                    remaining = chars.as_str();
                    continue;
                }
                break;
            }
            self.result.push(chunk.to_string());
            remaining = rest;
        }
    }

    fn flush(&mut self) {
        if !self.current.is_empty() {
            self.result.push(std::mem::take(&mut self.current));
            self.width = 0;
        }
    }

    fn finish(mut self, text: &str) -> Vec<String> {
        if !self.current.is_empty() {
            self.result.push(self.current);
        }
        if self.result.is_empty() && text.is_empty() {
            self.result.push(String::new());
        }
        self.result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_wrap_empty_yields_one_empty_line() {
        let lines = word_wrap("", 10, 10);
        assert_eq!(lines, vec![""]);
    }

    #[test]
    fn word_wrap_splits_long_word() {
        let lines = word_wrap("abcdefghij", 3, 3);
        assert_eq!(lines, vec!["abc", "def", "ghi", "j"]);
    }

    #[test]
    fn wrap_respects_display_width() {
        // Each CJK character is two display cells; wrapping at width 4 should
        // keep pairs together instead of counting characters.
        let lines = word_wrap("日本語テキスト", 4, 4);
        assert_eq!(lines, vec!["日本", "語テ", "キス", "ト"]);
    }

    #[test]
    fn element_line_count_spacer_is_one() {
        assert_eq!(element_line_count(&Element::spacer().at(0.0), 80), 1);
    }

    #[test]
    fn user_message_line_count_matches_wide_viewport() {
        let msg = Element::user("hello").at(0.0);
        // Margins (2) + one content line (1) = 3.
        assert_eq!(element_line_count(&msg, 80), 3);
    }

    #[test]
    fn user_message_wraps_narrow_viewport() {
        let msg = Element::user("hello world from runie").at(0.0);
        let count = element_line_count(&msg, 20);
        // Width 20 forces wrapping; should be > 3.
        assert!(count > 3, "expected wrapping, got {count}");
    }

    #[test]
    fn thought_marker_wraps_narrow_viewport() {
        let thought = Element::thought("this is a long thought that should wrap").at(0.0);
        let count = element_line_count(&thought, 20);
        assert!(count > 1, "expected wrapping, got {count}");
    }
}
