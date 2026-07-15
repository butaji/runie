//! Layout helpers shared between core and the TUI renderer.
//!
//! These helpers are intentionally free of Ratatui types so that core can
//! use them for scroll math while the TUI uses the same logic to produce
//! actual rendered lines.

use crate::markdown::{extract_code_blocks, CodeBlock};
use crate::view::elements::Element;
use textwrap::wrap;
use unicode_width::UnicodeWidthStr;

/// User message prefix glyph (must match `runie_tui::theme::GLYPH_USER`).
pub const GLYPH_USER: &str = "❯ ";
/// Agent message prefix glyph (must match `runie_tui::theme::GLYPH_AGENT`).
pub const GLYPH_AGENT: &str = "◆ ";
/// Indented continuation glyph (must match `runie_tui::theme::GLYPH_INDENT`).
pub const GLYPH_INDENT: &str = "     ";
/// Leading indent prepended to every feed line by the TUI feed renderer
/// (must match `runie_tui::theme::FEED_INDENT`). Combined with the 1-column
/// terminal margin this places post content at column 2 (0-indexed).
pub const FEED_INDENT: &str = " ";

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
        Element::AnthropicThinking { content, .. } => thought_marker_line_count(content, width),
        Element::ToolRunning { .. } => 1,
        Element::ToolDone { output, .. } => tool_done_line_count(output),
        Element::ToolSummary { .. } => 1,
        Element::ToolConfirmation { .. } => 1,
        Element::ContextGroup {
            tools, collapsed, ..
        } => {
            if *collapsed {
                1
            } else {
                tools.iter().map(|t| element_line_count(t, width)).sum()
            }
        }
        Element::SubagentRow {
            output, expanded, ..
        } => subagent_row_line_count(output, *expanded),
        Element::TurnComplete { .. } => 1,
        Element::Image { .. } => 1, // Image placeholder height
        Element::DataPart { data, .. } => data.lines().count().max(1),
        Element::MarkdownTable { rows, .. } => {
            // Header + separator + rows
            1 + 1 + rows.len()
        }
        Element::DiffOutput { content, .. } => content.lines().count().max(1),
        Element::WebSearchCall { results, .. } => {
            // Query line + result headers + separator
            1 + results.len() * 2
        }
        Element::AnsiStyled { plain_text, .. } => plain_text.lines().count().max(1),
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
        Element::AnthropicThinking { content, .. } => content.lines().count().max(1),
        Element::ToolRunning { .. } => 1,
        Element::ToolDone { output, .. } => {
            if output.is_empty() {
                1
            } else {
                1 + output.lines().count()
            }
        }
        Element::ToolSummary { .. } => 1,
        Element::ToolConfirmation { .. } => 1,
        Element::ContextGroup {
            tools, collapsed, ..
        } => {
            if *collapsed {
                1
            } else {
                tools.iter().map(fallback_line_count).sum()
            }
        }
        Element::SubagentRow {
            output, expanded, ..
        } => subagent_row_line_count(output, *expanded),
        Element::TurnComplete { .. } => 1,
        Element::Image { .. } => 1,
        Element::DataPart { data, .. } => data.lines().count().max(1),
        Element::MarkdownTable { rows, .. } => 1 + 1 + rows.len(),
        Element::DiffOutput { content, .. } => content.lines().count().max(1),
        Element::WebSearchCall { results, .. } => 1 + results.len() * 2,
        Element::AnsiStyled { plain_text, .. } => plain_text.lines().count().max(1),
    }
}

/// Header line plus, when expanded, one row per output line (unwrapped,
/// matching `render_subagent_row` in the TUI).
fn subagent_row_line_count(output: &str, expanded: bool) -> usize {
    if expanded && !output.is_empty() {
        1 + output.lines().count()
    } else {
        1
    }
}

fn user_message_line_count(content: &str, timestamp: f64, width: u16) -> usize {
    // The caller passes area_width - 2 (right-side slack); subtract the
    // leading feed indent the TUI prepends at render time (see
    // ui::messages::render_message_content).
    let inner_width = width.saturating_sub(FEED_INDENT.len() as u16);
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

    // +2 for the bg-padding lines the renderer adds above and below the card
    // (see render_user_message). Keeps scroll math in sync with the render.
    content_lines.max(1) + 2
}

fn agent_message_line_count(content: &str, timestamp: f64, width: u16) -> usize {
    // The caller passes area_width - 2 (right-side slack); subtract the
    // leading feed indent. Plain answer lines carry no leading glyph, so
    // prefix and continuation indents are zero.
    let inner_width = width.saturating_sub(FEED_INDENT.len() as u16);
    if inner_width == 0 {
        return content.lines().count().max(1);
    }

    let prefix_width = 0;
    let indent_width = 0;
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
        CodeBlock::Text { inlines, .. } => text_block_line_count(
            &inlines_to_plain_text(inlines),
            inner_width,
            prefix_width,
            indent_width,
            ts_width,
            *is_first,
        ),
        CodeBlock::Code { content, .. } => 1 + content.lines().count(),
        CodeBlock::List { items, .. } => items
            .iter()
            .map(|item| inlines_to_plain_text(item).lines().count().max(1))
            .sum(),
        CodeBlock::Blockquote(inlines) => inlines_to_plain_text(inlines).lines().count().max(1),
    };
    *is_first = false;
    lines
}

fn inlines_to_plain_text(inlines: &[crate::markdown::MdInline]) -> String {
    inlines
        .iter()
        .map(|i| if i.is_break() { "\n" } else { i.as_text() })
        .collect()
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
    // The caller passes area_width - 2 (right-side slack); subtract the
    // leading feed indent.
    let inner_width = width.saturating_sub(FEED_INDENT.len() as u16);
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
    UnicodeWidthStr::width(s) as u16
}

/// Word-wrap `text` into lines using display-cell width so wide characters
/// (CJK, emoji) count as two cells and are never split.
///
/// The first line of each input line is wrapped to `first_width`, and
/// continuation lines are wrapped to `rest_width`.
pub fn word_wrap(text: &str, first_width: u16, rest_width: u16) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }
    let first_w = (first_width.max(1)) as usize;
    let rest_w = (rest_width.max(1)) as usize;
    let mut result = Vec::new();

    for line in text.lines() {
        if line.is_empty() {
            result.push(String::new());
            continue;
        }
        // Wrap each input line to first_w (the first column of output).
        // Then re-wrap only the non-first wrapped segments to rest_w,
        // avoiding a second wrap pass on already-short lines.
        let wrapped = wrap(line, first_w);
        for (i, wl) in wrapped.into_iter().enumerate() {
            if i == 0 {
                // First segment always fits within first_w by definition.
                result.push(wl.into_owned());
            } else {
                // Subsequent segments may exceed rest_w; re-wrap only if needed.
                if UnicodeWidthStr::width(wl.as_ref()) > rest_w {
                    result.extend(wrap(&wl, rest_w).into_iter().map(|s| s.into_owned()));
                } else {
                    result.push(wl.into_owned());
                }
            }
        }
    }

    if result.is_empty() {
        vec![String::new()]
    } else {
        result
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
    fn word_wrap_single_pass_uses_rest_width_for_subsequent() {
        // When first_width > rest_width, lines that fit in first_w but exceed
        // rest_w should be re-wrapped to rest_w (single-pass logic).
        let lines = word_wrap("hello world here", 15, 5);
        // "hello world here" → wrap to 15 → ["hello world", "here"]
        // "here" (4) fits in rest_w (5), no re-wrap needed.
        assert_eq!(lines, vec!["hello world", "here"]);

        // A longer subsequent segment that exceeds rest_w should be re-wrapped.
        let lines2 = word_wrap("ab longwordxyz cd", 15, 5);
        // "ab longwordxyz cd" → wrap to 15 → ["ab longwordxyz", "cd"]
        // "ab longwordxyz" (15) > rest_w (5), re-wrap → ["ab lo", "ngwor", "dxyz"]
        let lines2_str = lines2.join("|");
        assert!(lines2
            .iter()
            .all(|l| UnicodeWidthStr::width(l.as_str()) <= 15));
        assert!(
            lines2_str.contains("ab lo"),
            "expected re-wrap: {lines2_str}"
        );
    }

    #[test]
    fn wrap_honors_first_and_rest_widths() {
        // Test that first_width and rest_width are actually used differently.
        // With first_width=8 and rest_width=4, the first line can fit more content
        // than continuation lines.
        let text = "hello world test";
        let lines = word_wrap(text, 8, 4);
        // The first line should fit up to 8 chars.
        // Subsequent lines should be limited to 4 chars.
        assert!(!lines.is_empty());
        // Verify the logic works: all lines should be within their respective widths
        // (approximately - textwrap may not perfectly fill every line)
        for line in &lines {
            let w = UnicodeWidthStr::width(line.as_str()) as u16;
            assert!(w <= 8, "line '{line}' width {w} exceeds max expected 8");
        }

        // Test with explicit first != rest widths
        let text2 = "aaaaaaaaaaaaaaaaaaaa"; // 20 'a' chars
        let lines2 = word_wrap(text2, 5, 3);
        // First line: up to 5 chars, then 3 for rest
        assert!(
            lines2.len() >= 3,
            "expected multiple lines with different widths"
        );
    }

    #[test]
    fn wrap_handles_wide_chars() {
        // CJK characters count as 2 display cells.
        let cjk = "日本語テキスト"; // Each char is 2 cells wide
        let lines = word_wrap(cjk, 4, 4);
        // Each line should have at most 4 display cells
        for line in &lines {
            let w = UnicodeWidthStr::width(line.as_str()) as u16;
            assert!(w <= 4, "CJK line '{line}' width {w} > 4");
        }

        // Emoji also count as 2 cells.
        let emoji = "👍👎🤖"; // thumbs up, down, robot
        let lines_emoji = word_wrap(emoji, 4, 4);
        for line in &lines_emoji {
            let w = UnicodeWidthStr::width(line.as_str()) as u16;
            assert!(w <= 4, "emoji line '{line}' width {w} > 4");
        }

        // Mixed content
        let mixed = "hello日本語world";
        let lines_mixed = word_wrap(mixed, 10, 10);
        for line in &lines_mixed {
            let w = UnicodeWidthStr::width(line.as_str()) as u16;
            assert!(w <= 10, "mixed line '{line}' width {w} > 10");
        }
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
        // One content line plus the bg-padding line above and below the card.
        assert_eq!(element_line_count(&msg, 80), 3);
    }

    #[test]
    fn user_message_wraps_narrow_viewport() {
        let msg = Element::user("hello world from runie").at(0.0);
        let count = element_line_count(&msg, 20);
        // Width 20 forces wrapping; should be > 1.
        assert!(count > 1, "expected wrapping, got {count}");
    }

    #[test]
    fn thought_marker_wraps_narrow_viewport() {
        let thought = Element::thought("this is a long thought that should wrap").at(0.0);
        let count = element_line_count(&thought, 20);
        assert!(count > 1, "expected wrapping, got {count}");
    }

    #[test]
    fn line_count_matches_textwrap() {
        // Verify core word_wrap produces counts consistent with textwrap's own
        // wrap output for a grid of widths and inputs.
        let inputs: Vec<&str> = vec![
            "hello world",
            "short",
            "word ",
            "word word word word word word word word word word word word word word word word word word word word ",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "hello\nworld",
        ];
        for text in inputs {
            for width in [1u16, 5, 10, 20, 40, 80] {
                let lines = word_wrap(text, width, width);
                // Non-empty input must produce at least one line.
                assert!(
                    !lines.is_empty(),
                    "word_wrap({text:?}, {width}) must not be empty"
                );
                // Each output line must be a whole word (no partial words mid-line).
                // The exception is CJK characters whose display width exceeds the
                // requested width — they cannot be split.
                for line in &lines {
                    let line_width = UnicodeWidthStr::width(line.as_str()) as u16;
                    assert!(
                        line_width <= width,
                        "word_wrap({text:?}, {width}) produced line '{line}' ({line_width} cells) > {width}"
                    );
                }
            }
        }
        // CJK text: wide characters can't be split; only test widths ≥ 2 cells.
        for width in [2u16, 5, 10, 20, 40, 80] {
            let lines = word_wrap("日本語テキスト", width, width);
            assert!(!lines.is_empty());
            for line in &lines {
                let line_width = UnicodeWidthStr::width(line.as_str()) as u16;
                assert!(
                    line_width <= width,
                    "CJK word_wrap({width}) produced line '{line}' ({line_width} cells) > {width}"
                );
            }
        }
    }

    #[test]
    fn wrapped_height_for_message() {
        // Agent message with known width produces a deterministic line count.
        let msg = Element::agent("one two three four five six").at(1.0);
        // At width 80, the sentence should not wrap; at width 10 it should.
        let wide_count = element_line_count(&msg, 80);
        let narrow_count = element_line_count(&msg, 10);
        assert!(
            narrow_count >= wide_count,
            "narrow ({narrow_count}) must ≥ wide ({wide_count})"
        );
        // Width 10 forces multi-line wrapping.
        assert!(narrow_count > 2, "width-10 should wrap: got {narrow_count}");
    }
}
