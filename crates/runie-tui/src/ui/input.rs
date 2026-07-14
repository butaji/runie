//! Input box rendering.
//!
//! This module replaces the custom input box rendering with ratatui-textarea
//! while maintaining compatibility with the existing InputActor architecture.
//!
//! Architecture:
//! - InputActor owns the authoritative text state
//! - UiActor projects state to Snapshot
//! - This module renders the input box using ratatui-textarea for text display
//!   while adding custom styling for the box (chevron, placeholder, etc.)

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Block,
    Frame,
};
use runie_core::Snapshot;

use crate::theme::{
    block_input, style_agent, style_chevron, style_hint, style_input_cursor,
    style_input_cursor_disabled, GLYPH_USER,
};

/// Render the input box.
/// This function bridges the Snapshot-based state with ratatui-textarea's rendering
/// while maintaining custom styling (chevron, placeholder, ghost completion, etc.).
pub(crate) fn input(f: &mut Frame, snap: &Snapshot, area: Rect) {
    if !snap.has_models {
        return;
    }

    // Unified "input enabled" flag: the input box is enabled only when
    // the user is actively typing in it. It is disabled in BOTH vim nav
    // mode AND when a dialog (command bar, model selector, etc.) is open.
    let token_held = !snap.vim_nav_mode && snap.dialog.is_none();

    let title = format!(" {} ", snap.input_title);
    let block = block_input(&title, snap.input_flash > 0);

    // Build the input content based on whether it's empty or has content.
    // `input_display` is the render view: labeled chips (e.g.
    // `[Pasted: 4 lines]`) replace their buffer span here.
    if snap.input_display.is_empty() {
        // Empty input: show placeholder
        render_empty_input(f, snap, area, &block, token_held);
    } else {
        // Has content: render with cursor
        render_input_content(f, snap, area, &block, token_held);
    }
}

/// Render the empty input state with cursor only (no placeholder).
/// Matches grok's behavior: shows "❯ " with a blinking cursor when focused.
fn render_empty_input(
    f: &mut Frame,
    snap: &Snapshot,
    area: Rect,
    block: &Block<'_>,
    token_held: bool,
) {
    let chevron_style = style_chevron(token_held);
    let cursor_style = if token_held {
        style_input_cursor()
    } else {
        style_input_cursor_disabled()
    };

    // Build line with chevron and cursor (no placeholder text, matching grok)
    let mut spans = vec![Span::styled(GLYPH_USER, chevron_style)];
    if token_held {
        spans.push(Span::styled(" ", cursor_style));
    }

    // Add image attachment label if present
    if let Some(label) = image_attachment_label(snap) {
        spans.push(Span::styled(label, style_hint()));
    }

    let line = Line::from(spans);

    let paragraph = ratatui::widgets::Paragraph::new(line).block(block.clone());

    f.render_widget(paragraph, area);
}

/// Render input content with cursor.
fn render_input_content(
    f: &mut Frame,
    snap: &Snapshot,
    area: Rect,
    block: &Block<'_>,
    token_held: bool,
) {
    let chevron_style = style_chevron(token_held);
    let text_style = if token_held {
        style_agent()
    } else {
        style_hint()
    };
    let cursor_style = if token_held {
        style_input_cursor()
    } else {
        style_input_cursor_disabled()
    };

    // Build lines with chevron prefix for first line, indentation for others
    let lines: Vec<Line<'static>> = snap
        .input_display
        .lines()
        .enumerate()
        .map(|(idx, line_content)| {
            let prefix: String = if idx == 0 {
                GLYPH_USER.to_string()
            } else {
                "  ".to_string()
            };

            if idx == 0 && !snap.ghost_completion.as_deref().unwrap_or("").is_empty() {
                // First line with ghost completion
                build_line_with_cursor_and_ghost_owned(
                    line_content,
                    prefix,
                    snap.cursor_display,
                    snap.ghost_completion.clone().unwrap_or_default(),
                    chevron_style,
                    text_style,
                    cursor_style,
                )
            } else if idx == input_cursor_line(snap) && token_held {
                // Cursor is on this line
                build_line_with_cursor_owned(
                    line_content,
                    prefix,
                    input_cursor_col_in_line(snap),
                    text_style,
                    cursor_style,
                )
            } else {
                // Regular line without cursor
                Line::from(vec![
                    Span::styled(prefix, chevron_style),
                    Span::styled(line_content.to_string(), text_style),
                ])
            }
        })
        .collect();

    // Add trailing cursor line if cursor is after last newline
    let mut all_lines = lines;
    let cursor_line_idx = input_cursor_line(snap);
    let total_input_lines = snap.input_display.lines().count();
    if cursor_line_idx >= total_input_lines {
        all_lines.push(build_trailing_cursor_line(
            &chevron_style,
            snap.input_display.is_empty(),
            token_held,
        ));
    }

    // Handle scrolling
    let total_lines = all_lines.len();
    let inner_height = area.height.saturating_sub(2) as usize;
    let scroll = snap.input_scroll.min(total_lines.saturating_sub(1));
    let visible_lines: Vec<Line<'_>> = if total_lines > inner_height {
        let end = (scroll + inner_height).min(total_lines);
        all_lines[scroll..end].to_vec()
    } else {
        all_lines
    };

    let paragraph = ratatui::widgets::Paragraph::new(visible_lines)
        .block(block.clone())
        .wrap(ratatui::widgets::Wrap { trim: false });

    f.render_widget(paragraph, area);

    // Render scrollbar if needed
    if total_lines > inner_height {
        render_input_scrollbar(f, area, total_lines, scroll, inner_height);
    }
}

/// Build a line with cursor rendering (owned strings).
fn build_line_with_cursor_owned(
    content: &str,
    prefix: String,
    col: usize,
    text_style: Style,
    cursor_style: Style,
) -> Line<'static> {
    let boundary = content.floor_char_boundary(col);
    let before = &content[..boundary];
    let (at_cursor, after) = if boundary < content.len() {
        let c = content[boundary..].chars().next().unwrap();
        let char_len = c.len_utf8();
        (c.to_string(), content[boundary + char_len..].to_string())
    } else {
        (" ".to_string(), String::new())
    };

    Line::from(vec![
        Span::styled(prefix, style_chevron(true)),
        Span::styled(before.to_string(), text_style),
        Span::styled(at_cursor, cursor_style),
        Span::styled(after, text_style),
    ])
}

/// Build a line with cursor and ghost completion (owned strings).
fn build_line_with_cursor_and_ghost_owned(
    content: &str,
    prefix: String,
    cursor_pos: usize,
    ghost: String,
    chevron_style: Style,
    text_style: Style,
    cursor_style: Style,
) -> Line<'static> {
    let boundary = content.floor_char_boundary(cursor_pos);
    let before = &content[..boundary];
    let (at_cursor, after) = if boundary < content.len() {
        let c = content[boundary..].chars().next().unwrap();
        let char_len = c.len_utf8();
        (c.to_string(), content[boundary + char_len..].to_string())
    } else {
        (" ".to_string(), String::new())
    };

    Line::from(vec![
        Span::styled(prefix, chevron_style),
        Span::styled(before.to_string(), text_style),
        Span::styled(at_cursor, cursor_style),
        Span::styled(after, text_style),
        Span::styled(ghost, style_hint()),
        Span::styled("→", style_hint()),
    ])
}

/// Build a trailing cursor line (shown when cursor is after the last character).
fn build_trailing_cursor_line(
    chevron_style: &Style,
    is_empty: bool,
    token_held: bool,
) -> Line<'static> {
    let prefix = if is_empty { GLYPH_USER } else { "  " };
    let mut spans = vec![Span::styled(prefix, *chevron_style)];
    if token_held {
        spans.push(Span::styled(" ", style_input_cursor()));
    }
    Line::from(spans)
}

/// Calculate the line index where the cursor is positioned.
fn input_cursor_line(snap: &Snapshot) -> usize {
    let pos = snap.cursor_display.min(snap.input_display.len());
    snap.input_display[..pos]
        .chars()
        .filter(|&c| c == '\n')
        .count()
}

/// Calculate the column position within the current line.
fn input_cursor_col_in_line(snap: &Snapshot) -> usize {
    let pos = snap.cursor_display.min(snap.input_display.len());
    let line_idx = input_cursor_line(snap);
    pos - snap
        .input_display
        .lines()
        .take(line_idx)
        .map(|l| l.len() + 1)
        .sum::<usize>()
}

/// Generate image attachment label if any.
fn image_attachment_label(snap: &Snapshot) -> Option<String> {
    match snap.image_attachments.len() {
        0 => None,
        1 => Some(" 📎 1 image".to_owned()),
        n => Some(format!(" 📎 {} images", n)),
    }
}

/// Render scrollbar for multi-line input.
fn render_input_scrollbar(f: &mut Frame, area: Rect, total: usize, scroll: usize, height: usize) {
    let sb_area = Rect {
        x: area.x + area.width.saturating_sub(1),
        y: area.y + 1,
        width: 1,
        height: area.height.saturating_sub(2),
    };
    super::render_scrollbar(f, sb_area, total, scroll as u16, height);
}

/// Count the number of visual lines needed for the input.
pub fn count_input_lines(input: &str) -> usize {
    if input.is_empty() {
        return 1;
    }
    let mut lines = input.lines().count().max(1);
    if input.ends_with('\n') {
        lines += 1;
    }
    lines
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;

    #[test]
    fn cursor_line_calculation() {
        let mut snap = Snapshot::default();
        snap.input_display = "hello".to_string();
        snap.cursor_display = 2;
        assert_eq!(input_cursor_line(&snap), 0);

        snap.input_display = "line1\nline2".to_string();
        snap.cursor_display = 6; // at 'l' of line2
        assert_eq!(input_cursor_line(&snap), 1);

        snap.cursor_display = 7; // at 'i' of line2
        assert_eq!(input_cursor_line(&snap), 1);
    }

    #[test]
    fn cursor_col_in_line() {
        let mut snap = Snapshot::default();
        snap.input_display = "hello".to_string();
        snap.cursor_display = 2;
        assert_eq!(input_cursor_col_in_line(&snap), 2);

        snap.input_display = "line1\nline2".to_string();
        snap.cursor_display = 6; // at 'l' of line2
        assert_eq!(input_cursor_col_in_line(&snap), 0);

        snap.cursor_display = 7; // at 'i' of line2
        assert_eq!(input_cursor_col_in_line(&snap), 1);
    }

    #[test]
    fn cursor_helpers_use_display_coordinates() {
        // A labeled chip shrinks the rendered text: cursor mapping must use
        // display coordinates, not buffer coordinates.
        let mut snap = Snapshot::default();
        snap.input = "xa\nb\nc\nd".to_string();
        snap.cursor_pos = 9;
        snap.input_display = "x[Pasted: 4 lines]".to_string();
        snap.cursor_display = "x[Pasted: 4 lines]".len();
        assert_eq!(input_cursor_line(&snap), 0);
        assert_eq!(input_cursor_col_in_line(&snap), 18);
    }

    #[test]
    fn count_input_lines_empty() {
        assert_eq!(count_input_lines(""), 1);
    }

    #[test]
    fn count_input_lines_single() {
        assert_eq!(count_input_lines("hello"), 1);
    }

    #[test]
    fn count_input_lines_multi() {
        assert_eq!(count_input_lines("line1\nline2"), 2);
    }

    #[test]
    fn count_input_lines_trailing_newline() {
        assert_eq!(count_input_lines("line1\n"), 2);
    }

    #[test]
    fn render_cursor_spans_clamps_to_char_boundary() {
        // "é" is two bytes. A cursor at byte 1 used to slice in the middle
        // of the character and panic.
        let line = build_line_with_cursor_owned(
            "café",
            "❯ ".to_string(),
            4,
            style_agent(),
            style_input_cursor(),
        );
        assert_eq!(line.spans.len(), 4);
    }

    #[test]
    fn render_cursor_spans_does_not_panic_in_mid_character() {
        // Byte 3 is inside the two-byte "é" (bytes 3-4). floor_char_boundary
        // snaps it back to byte 3, the start of "é".
        let line = build_line_with_cursor_owned(
            "café",
            "❯ ".to_string(),
            3,
            style_agent(),
            style_input_cursor(),
        );
        assert_eq!(line.spans.len(), 4);
    }
}
