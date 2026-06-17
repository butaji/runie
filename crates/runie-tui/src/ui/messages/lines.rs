use ratatui::text::Line;
use runie_core::Element;

use crate::ui::render_lines::to_lines_and_count;

/// Flatten the feed into wrapped terminal lines and record which element
/// each visible row belongs to. This mapping is what the vim-nav selection
/// highlight uses, so the highlight height matches the *rendered* height of
/// a post even when message text wraps.
pub(crate) fn build_lines_with_mapping(
    snap: &runie_core::Snapshot,
    content_width: u16,
) -> (Vec<Line<'_>>, Vec<usize>) {
    let mut lines = Vec::with_capacity(snap.total_lines);
    let mut mapping = Vec::with_capacity(snap.total_lines);
    for (idx, elem) in snap.elements.iter().enumerate() {
        let (elem_lines, wrapped_rows) = to_lines_and_count(elem, content_width);
        mapping.extend(std::iter::repeat_n(idx, wrapped_rows));
        lines.extend(elem_lines);
    }

    // Append streaming tail when a turn is active
    if snap.turn_active && !snap.streaming_tail.is_empty() {
        let streaming_lines = wrap_text_to_lines(&snap.streaming_tail, content_width);
        let last_idx = snap.elements.len().saturating_sub(1);
        for line in streaming_lines {
            mapping.push(last_idx);
            lines.push(line);
        }
    }

    (lines, mapping)
}

/// Wrap text to lines respecting content width.
fn wrap_text_to_lines(text: &str, width: u16) -> Vec<Line<'_>> {
    use std::borrow::Cow;

    let mut result = Vec::new();
    for line in text.lines() {
        if line.is_empty() {
            result.push(Line::from(""));
            continue;
        }

        // Simple word wrap at width boundary
        let chars_per_line = width as usize;
        let chars: Vec<char> = line.chars().collect();

        if chars.len() <= chars_per_line {
            result.push(Line::from(Cow::Borrowed(line)));
            continue;
        }

        // Break into chunks of chars_per_line
        for chunk in chars.chunks(chars_per_line) {
            let wrapped: String = chunk.iter().collect();
            result.push(Line::from(Cow::Owned(wrapped)));
        }
    }

    result
}

pub(crate) fn estimate_element_tokens(elem: &Element) -> usize {
    use runie_core::Element::*;
    match elem {
        UserMessage { content, .. }
        | AgentMessage { content, .. }
        | ThoughtMarker { content, .. } => content.len() / 4,
        Thinking { .. } | ThoughtSummary { .. } | ToolSummary { .. } | TurnComplete { .. } => 10,
        ToolRunning { .. } => 10,
        ToolDone { output, .. } => output.len() / 4 + 10,
        Spacer { .. } => 0,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::ui::render_lines::{element_line_count, to_lines_internal};
    use ratatui::{backend::TestBackend, Terminal, widgets::Paragraph};
    use runie_core::ui::elements::Element;

    fn assert_count_matches(element: Element, width: u16) {
        let count = element_line_count(&element, width);
        let rendered = to_lines_internal(&element, width);
        let wrapped_rows = Paragraph::new(rendered.as_slice())
            .wrap(ratatui::widgets::Wrap { trim: false })
            .line_count(width);
        let core_count = runie_core::layout::element_line_count(&element, width);
        assert_eq!(
            count,
            wrapped_rows,
            "element_line_count mismatch for {element:?} at width {width}: expected {wrapped_rows}, got {count}",
        );
        assert_eq!(
            core_count,
            wrapped_rows,
            "core layout count mismatch for {element:?} at width {width}: expected {wrapped_rows}, got {core_count}",
        );
    }

    #[test]
    fn element_line_count_matches_rendered_lines_user_message() {
        assert_count_matches(Element::user("hello world").at(0.0), 80);
        assert_count_matches(Element::user("a".repeat(200)).at(0.0), 40);
        assert_count_matches(Element::user("line1\nline2\nline3").at(0.0), 30);
    }

    #[test]
    fn element_line_count_matches_rendered_lines_agent_message() {
        assert_count_matches(Element::agent("hello world").at(0.0), 80);
        assert_count_matches(Element::agent("a".repeat(200)).at(0.0), 40);
        assert_count_matches(Element::agent("line1\nline2\nline3").at(0.0), 30);
    }

    #[test]
    fn ast_line_count_matches_render() {
        assert_count_matches(Element::agent("plain **bold** _italic_ `code` text").at(0.0), 80);
        assert_count_matches(Element::agent(&"**bold** word ".repeat(20)).at(0.0), 40);
        assert_count_matches(
            Element::agent("line with `code` and **bold**\nnext line with *italic*").at(0.0),
            50,
        );
    }

    #[test]
    fn agent_message_with_code_block_line_count_matches_rendered_rows() {
        assert_count_matches(
            Element::agent("intro\n```rust\nlet x = 1;\n```\noutro").at(0.0),
            80,
        );
        assert_count_matches(
            Element::agent("```python\nprint('hello')\nprint('world')\n```").at(0.0),
            40,
        );
    }

    #[test]
    fn agent_message_with_list_line_count_matches_rendered_rows() {
        assert_count_matches(Element::agent("items:\n- one\n- two").at(0.0), 80);
        assert_count_matches(Element::agent("1. first\n2. second").at(0.0), 40);
    }

    #[test]
    fn wide_text_does_not_overflow_viewport() {
        // Each CJK character is two display cells; ensure core and renderer agree.
        assert_count_matches(Element::agent("日本語テキスト").at(0.0), 20);
        assert_count_matches(Element::user("日本語テキスト").at(0.0), 20);
    }

    #[test]
    fn element_line_count_matches_rendered_lines_thought_marker() {
        assert_count_matches(Element::thought("hello world").at(0.0), 80);
        assert_count_matches(Element::thought("a".repeat(200)).at(0.0), 40);
        assert_count_matches(Element::thought("line1\nline2\nline3").at(0.0), 30);
    }

    #[test]
    fn element_line_count_matches_rendered_lines_simple_variants() {
        let started = std::time::Instant::now();
        assert_count_matches(Element::spacer().at(0.0), 80);
        assert_count_matches(Element::thinking(started).at(0.0), 80);
        assert_count_matches(Element::thought_summary("summary", 1.0).at(0.0), 80);
        assert_count_matches(Element::tool_running("ls", ".", started).at(0.0), 80);
        assert_count_matches(
            Element::tool_done("ls", ".", 0.5, "out1\nout2\nout3", None, false).at(0.0),
            80,
        );
        assert_count_matches(Element::tool_summary("ls", 0.5).at(0.0), 80);
        assert_count_matches(Element::turn_complete(1.0).at(0.0), 80);
    }
}
