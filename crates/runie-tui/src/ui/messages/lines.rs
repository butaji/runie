use ratatui::text::Line;
use runie_core::{layout::word_wrap, Element};

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

/// Wrap streaming tail text to terminal lines, using the shared `word_wrap`
/// from runie-core so wrapping rules stay in sync with core scroll math.
fn wrap_text_to_lines(text: &str, width: u16) -> Vec<Line<'_>> {
    word_wrap(text, width, width)
        .into_iter()
        .map(Line::from)
        .collect()
}

pub(crate) fn estimate_element_tokens(elem: &Element) -> usize {
    use runie_core::Element::*;
    match elem {
        UserMessage { content, .. }
        | AgentMessage { content, .. }
        | ThoughtMarker { content, .. }
        | AnthropicThinking { content, .. } => content.len() / 4,
        Thinking { .. } | ThoughtSummary { .. } | ToolSummary { .. } | TurnComplete { .. } => 10,
        ToolRunning { .. } => 10,
        ToolDone { output, .. } => output.len() / 4 + 10,
        SubagentRow { output, .. } => output.len() / 4 + 10,
        ToolConfirmation { args, .. } => args.len() / 4,
        ContextGroup { tools, .. } => tools.iter().map(estimate_element_tokens).sum(),
        Spacer { .. } => 0,
        Image { data, .. } => data.len() / 4, // Approximate token count for base64
        DataPart { data, .. } => data.len() / 4,
        MarkdownTable { headers, rows, .. } => {
            let header_len: usize = headers.iter().map(|h| h.len()).sum();
            let row_len: usize = rows.iter().map(|r| r.iter().map(|c| c.len()).sum::<usize>()).sum();
            (header_len + row_len) / 4
        }
        DiffOutput { content, .. } => content.len() / 4,
        WebSearchCall { query, results, .. } => {
            query.len() / 4 + results.iter().map(|r| r.title.len() + r.snippet.len()).sum::<usize>() / 4
        }
        AnsiStyled { plain_text, .. } => plain_text.len() / 4,
        SubagentRow { .. } => 10,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::render_lines::{element_line_count, to_lines_internal};
    use ratatui::widgets::Paragraph;
    use runie_core::layout::word_wrap;
    use runie_core::view::elements::Element;

    #[test]
    fn wrap_text_to_lines_matches_word_wrap() {
        // Streaming tail uses word_wrap; verify the adapter produces the same lines.
        for width in [20u16, 40, 80] {
            let text = "hello world from runie agent";
            let direct: Vec<String> = word_wrap(text, width, width);
            let adapted: Vec<String> = wrap_text_to_lines(text, width)
                .into_iter()
                .map(|l| l.to_string())
                .collect();
            assert_eq!(direct, adapted, "width {width}");
        }
    }

    fn assert_count_matches(element: Element, width: u16) {
        let count = element_line_count(&element, width);
        let rendered = to_lines_internal(&element, width);
        // Verify that the pre-calculated count matches the actual rendered lines.
        // Note: We don't re-wrap with Paragraph since our custom wrapping already
        // handles the timestamp and indentation.
        assert_eq!(
            count,
            rendered.len(),
            "element_line_count mismatch for {element:?} at width {width}: expected {}, got {}",
            rendered.len(),
            count,
        );
    }

    #[test]
    fn element_line_count_matches_rendered_lines_user_message() {
        assert_count_matches(Element::user("hello world").at(0.0), 80);
        // With timestamp on first line, content wraps to full width (not reduced by ts_width)
        assert_count_matches(Element::user("a".repeat(200)).at(0.0), 40);
        assert_count_matches(Element::user("line1\nline2\nline3").at(0.0), 30);
    }

    #[test]
    fn element_line_count_matches_rendered_lines_agent_message() {
        assert_count_matches(Element::agent("hello world").at(0.0), 80);
        // With timestamp on first line, content wraps to full width (not reduced by ts_width)
        assert_count_matches(Element::agent("a".repeat(200)).at(0.0), 40);
        assert_count_matches(Element::agent("line1\nline2\nline3").at(0.0), 30);
    }

    #[test]
    fn ast_line_count_matches_render() {
        assert_count_matches(
            Element::agent("plain **bold** _italic_ `code` text").at(0.0),
            80,
        );
        assert_count_matches(
            Element::agent("**bold** word ".repeat(20).as_str()).at(0.0),
            40,
        );
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
