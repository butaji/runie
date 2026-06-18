//! Input box rendering.

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};
use runie_core::Snapshot;

use crate::theme::{
    block_input, style_chevron, style_hint, style_input_cursor, style_input_cursor_disabled,
    style_placeholder,
};

pub(crate) fn input(f: &mut Frame, snap: &Snapshot, area: Rect) {
    if !snap.has_models {
        return;
    }
    let title = format!(" {}/{} ", snap.provider, snap.model);
    let block = block_input(&title, snap.input_flash > 0);
    // Unified "input enabled" flag: the input box is enabled only when
    // the user is actively typing in it. It is disabled in BOTH vim nav
    // mode AND when a dialog (command bar, model selector, etc.) is
    // open. While disabled, no cursor is rendered and the chevron is
    // dimmed — this keeps the visual treatment consistent.
    let token_held = !snap.vim_nav_mode && snap.dialog.is_none();
    let lines = build_input_lines(snap, token_held);
    let total_lines = lines.len();
    let inner_height = area.height.saturating_sub(2) as usize;
    let show_scrollbar = total_lines > inner_height;

    let scroll = snap.input_scroll.min(total_lines.saturating_sub(1));
    let visible_lines = if total_lines > inner_height {
        let end = (scroll + inner_height).min(total_lines);
        lines[scroll..end].to_vec()
    } else {
        lines
    };

    f.render_widget(
        Paragraph::new(visible_lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );

    if show_scrollbar {
        render_input_scrollbar(f, area, total_lines, scroll, inner_height);
    }
}

fn render_input_scrollbar(f: &mut Frame, area: Rect, total: usize, scroll: usize, height: usize) {
    let sb_area = Rect {
        x: area.x + area.width.saturating_sub(1),
        y: area.y + 1,
        width: 1,
        height: area.height.saturating_sub(2),
    };
    super::render_scrollbar(f, sb_area, total, scroll as u16, height);
}

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

fn build_input_lines(snap: &Snapshot, token_held: bool) -> Vec<Line<'_>> {
    let chevron_style = style_chevron(token_held);
    if snap.input.is_empty() && !snap.placeholder.is_empty() {
        return vec![build_placeholder_line(snap, chevron_style, token_held)];
    }

    let cursor = input_cursor(snap);
    let mut result = build_input_content_lines(snap, cursor, chevron_style, token_held);
    if cursor.line_idx >= snap.input.lines().count() {
        result.push(build_trailing_cursor_line(
            snap,
            cursor,
            chevron_style,
            token_held,
        ));
    }
    result
}

fn build_placeholder_line(
    snap: &Snapshot,
    chevron_style: Style,
    token_held: bool,
) -> Line<'static> {
    let mut spans = vec![Span::styled(crate::theme::GLYPH_USER, chevron_style)];
    // No cursor block while the input box is disabled (vim nav mode or
    // a dialog is open). `token_held` is the unified enabled flag.
    if token_held {
        spans.push(Span::styled(" ".to_string(), style_input_cursor()));
    }
    spans.push(Span::styled(snap.placeholder.clone(), style_placeholder()));
    Line::from(spans)
}

#[derive(Copy, Clone)]
struct InputCursor {
    line_idx: usize,
    col_in_line: usize,
}

fn input_cursor(snap: &Snapshot) -> InputCursor {
    let pos = snap.cursor_pos.min(snap.input.len());
    let line_idx = snap.input[..pos].chars().filter(|&c| c == '\n').count();
    let col_in_line = pos
        - snap
            .input
            .lines()
            .take(line_idx)
            .map(|l| l.len() + 1)
            .sum::<usize>();
    InputCursor {
        line_idx,
        col_in_line,
    }
}

fn build_input_content_lines(
    snap: &Snapshot,
    cursor: InputCursor,
    chevron_style: Style,
    token_held: bool,
) -> Vec<Line<'_>> {
    let indent = "  ";
    let last_line_idx = snap.input.lines().count().saturating_sub(1);
    snap.input
        .lines()
        .enumerate()
        .map(|(line_idx, line_content)| {
            let prefix = if line_idx == 0 {
                crate::theme::GLYPH_USER
            } else {
                indent
            };
            let mut spans = vec![Span::styled(prefix, chevron_style)];
            spans.extend(line_spans(
                line_idx,
                line_content,
                cursor,
                token_held,
                snap,
                last_line_idx,
            ));
            if line_idx == 0 {
                if let Some(label) = image_attachment_label(snap) {
                    spans.push(Span::styled(label, style_hint()));
                }
            }
            Line::from(spans)
        })
        .collect()
}

fn line_spans<'a>(
    line_idx: usize,
    line_content: &'a str,
    cursor: InputCursor,
    token_held: bool,
    snap: &'a Snapshot,
    last_line_idx: usize,
) -> Vec<Span<'a>> {
    if line_idx == cursor.line_idx {
        let ghost = if line_idx == last_line_idx {
            snap.ghost_completion.as_deref().unwrap_or("")
        } else {
            ""
        };
        render_cursor_spans(line_content, cursor.col_in_line, token_held, ghost)
    } else {
        let text_style = if token_held {
            crate::theme::style_agent()
        } else {
            style_hint()
        };
        vec![Span::styled(line_content, text_style)]
    }
}

fn build_trailing_cursor_line(
    snap: &Snapshot,
    _cursor: InputCursor,
    chevron_style: Style,
    token_held: bool,
) -> Line<'static> {
    let prefix = if snap.input.is_empty() {
        crate::theme::GLYPH_USER
    } else {
        "  "
    };
    let mut spans = vec![Span::styled(prefix, chevron_style)];
    if token_held {
        spans.push(Span::styled(" ", style_input_cursor()));
    }
    Line::from(spans)
}

fn render_cursor_spans<'a>(
    line_content: &'a str,
    cursor_col_in_line: usize,
    token_held: bool,
    ghost: &'a str,
) -> Vec<Span<'a>> {
    // While the input box is disabled, render both the text and the
    // cursor with the dimmed disabled style so the whole field reads
    // as inactive.
    let (text_style, cursor_style) = if token_held {
        (crate::theme::style_agent(), style_input_cursor())
    } else {
        (style_hint(), style_input_cursor_disabled())
    };
    let boundary = line_content.floor_char_boundary(cursor_col_in_line);
    let before = &line_content[..boundary];
    let (at_cursor, after) = if boundary < line_content.len() {
        let c = line_content[boundary..].chars().next().unwrap();
        let char_len = c.len_utf8();
        (c.to_string(), &line_content[boundary + char_len..])
    } else if token_held {
        (" ".to_string(), "")
    } else {
        ("".to_string(), "")
    };
    let mut spans = vec![
        Span::styled(before, text_style),
        Span::styled(at_cursor, cursor_style),
        Span::styled(after, text_style),
    ];
    if !ghost.is_empty() {
        spans.push(Span::styled(ghost, style_hint()));
        spans.push(Span::styled("→", style_hint()));
    }
    spans
}

fn image_attachment_label(snap: &Snapshot) -> Option<String> {
    match snap.image_attachments.len() {
        0 => None,
        1 => Some(" 📎 1 image".to_string()),
        n => Some(format!(" 📎 {} images", n)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_cursor_spans_clamps_to_char_boundary() {
        // "é" is two bytes. A cursor at byte 1 used to slice in the middle
        // of the character and panic.
        let spans = render_cursor_spans("café", 4, true, "");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].content, "caf");
        assert_eq!(spans[1].content, "é");
        assert_eq!(spans[2].content, "");
    }

    #[test]
    fn render_cursor_spans_does_not_panic_in_mid_character() {
        // Byte 3 is inside the two-byte "é" (bytes 3-4). floor_char_boundary
        // snaps it back to byte 3, the start of "é".
        let spans = render_cursor_spans("café", 3, true, "");
        assert_eq!(spans[0].content, "caf");
        assert_eq!(spans[1].content, "é");
    }

    #[test]
    fn render_cursor_spans_renders_space_at_end_when_held() {
        let spans = render_cursor_spans("hi", 2, true, "");
        assert_eq!(spans[0].content, "hi");
        assert_eq!(spans[1].content, " ");
        assert_eq!(spans[2].content, "");
    }
}
