//! Message feed rendering and vim-nav selection highlight.

use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use runie_core::Snapshot;
use runie_core::Element;

pub(crate) mod lines;
pub(crate) mod nav;

pub(crate) use lines::{build_lines_with_mapping, estimate_element_tokens};

pub(crate) fn render_messages(f: &mut Frame, snap: &Snapshot, area: Rect) {
    if snap.elements.is_empty() {
        render_empty_state(f, area);
    } else {
        render_message_content(f, snap, area);
    }
}

fn render_empty_state(f: &mut Frame, area: Rect) {
    f.render_widget(Paragraph::new(""), area);
}

fn render_message_content(f: &mut Frame, snap: &Snapshot, area: Rect) {
    let height = area.height as usize;
    if height == 0 || snap.total_lines == 0 {
        return;
    }

    let content_width = area.width.saturating_sub(2);
    let (lines, row_to_element) = build_lines_with_mapping(snap, content_width);
    let offset = nav::compute_scroll_offset(snap, &row_to_element, area.height as usize);

    // Render lines with user message backgrounds applied directly to lines
    render_paragraph_with_user_backgrounds(f, snap, area, lines, offset, &row_to_element);

    if snap.vim_nav_mode {
        nav::highlight_selected_post(f, snap, area, &row_to_element, offset);
    }

    render_scrollbar_if_needed(f, area, row_to_element.len(), offset, height);
}

/// Render lines with user message backgrounds applied to the lines.
/// User messages and their adjacent spacers get full-width backgrounds.
fn render_paragraph_with_user_backgrounds(
    f: &mut Frame,
    snap: &Snapshot,
    area: Rect,
    lines: Vec<Line<'_>>,
    offset: u16,
    row_to_element: &[usize],
) {
    let height = area.height as usize;
    let start = offset as usize;
    let bg = crate::theme::color_bg_user();
    let visible_start = offset as usize;
    let full_width = f.area().width;

    // Build modified lines with user background applied
    let modified_lines: Vec<Line<'static>> = lines
        .iter()
        .skip(start)
        .take(height)
        .enumerate()
        .map(|(row_offset, line)| {
            let abs_row = visible_start + row_offset;
            let elem_idx = *row_to_element.get(abs_row).unwrap_or(&usize::MAX);
            let is_user_related = is_user_related_row(snap, elem_idx);

            if is_user_related {
                // Convert to owned line with background applied
                line_to_owned_with_bg(line, bg)
            } else {
                line_to_owned(line)
            }
        })
        .collect();

    // FIRST: Draw full-width backgrounds for user-related rows (for margins)
    for row_offset in 0..height {
        let row = area.y + row_offset as u16;
        let abs_row = visible_start + row_offset;
        let elem_idx = *row_to_element.get(abs_row).unwrap_or(&usize::MAX);
        let is_user_related = is_user_related_row(snap, elem_idx);

        if is_user_related {
            // Fill FULL width background from x=0 to terminal edge
            for x in 0..full_width {
                let cell = &mut f.buffer_mut()[(x, row)];
                let _ = cell.set_bg(bg);
            }
        }
    }

    // THEN: Render text on top of the backgrounds
    for (row_offset, line) in modified_lines.iter().enumerate() {
        let row = area.y + row_offset as u16;
        f.render_widget(
            ratatui::widgets::Paragraph::new(line.clone()),
            Rect::new(area.x, row, area.width, 1),
        );
    }
}

/// Convert a line to owned with background applied to all spans.
fn line_to_owned_with_bg(line: &Line<'_>, bg: ratatui::style::Color) -> Line<'static> {
    let spans: Vec<Span<'static>> = line
        .spans
        .iter()
        .map(|s| {
            let mut style = s.style;
            if style.bg.is_none() {
                style = style.bg(bg);
            }
            Span::styled(s.content.to_string(), style)
        })
        .collect();
    Line::from(spans)
}

/// Convert a line to owned.
fn line_to_owned(line: &Line<'_>) -> Line<'static> {
    let spans: Vec<Span<'static>> = line
        .spans
        .iter()
        .map(|s| Span::styled(s.content.to_string(), s.style))
        .collect();
    Line::from(spans)
}

/// Check if a row belongs to a user message card.
///
/// The bg.user background covers only the user message element's own rows
/// (its internal top/bottom padding plus content). The trailing spacer that
/// follows a user post stays on the normal feed background, forming the
/// margin line that separates the card from whatever comes next.
fn is_user_related_row(snap: &Snapshot, elem_idx: usize) -> bool {
    if elem_idx == usize::MAX {
        return false;
    }
    matches!(snap.elements.get(elem_idx), Some(Element::UserMessage { .. }))
}

fn render_paragraph(f: &mut Frame, area: Rect, lines: Vec<Line<'_>>, offset: u16) {
    let height = area.height as usize;
    let start = offset as usize;
    // Render lines directly into the buffer — skip Paragraph to avoid re-wrapping.
    for (row_offset, line) in lines.iter().skip(start).take(height).enumerate() {
        let row = area.y + row_offset as u16;
        f.render_widget(
            ratatui::widgets::Paragraph::new(line.clone()),
            Rect::new(area.x, row, area.width, 1),
        );
    }
}

fn render_scrollbar_if_needed(f: &mut Frame, area: Rect, total: usize, offset: u16, height: usize) {
    if total > height {
        let full_w = f.area().width;
        let scrollbar_area = Rect {
            x: (area.x + area.width).min(full_w.saturating_sub(1)),
            y: area.y,
            width: 1,
            height: area.height,
        };
        super::render_scrollbar(f, scrollbar_area, total, offset, height);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::ui::render_lines::to_lines_internal;
    use ratatui::{backend::TestBackend, Terminal};
    use runie_core::Element;

    #[test]
    fn scrollbar_thumb_matches_markdown_message_height() {
        let width = 40u16;
        let height = 4u16;
        let element =
            Element::agent("items:\n- one\n- two\n- three\n- four\n- five\n- six").at(0.0);
        let rendered = to_lines_internal(&element, width).len();
        assert!(
            rendered > height as usize,
            "message should be taller than viewport"
        );

        let snap = Snapshot {
            elements: Arc::new([element]),
            line_counts: Arc::new([rendered]),
            total_lines: rendered,
            last_visible_height: height,
            content_width: width,
            ..Default::default()
        };

        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| render_messages(f, &snap, f.area()))
            .unwrap();

        let thumb = crate::theme::SCROLLBAR_THUMB.chars().next().unwrap();
        let buffer = terminal.backend().buffer();
        let has_thumb = buffer
            .content()
            .iter()
            .any(|cell| cell.symbol() == thumb.to_string());
        assert!(
            has_thumb,
            "scrollbar thumb should be visible for tall message"
        );
    }

    /// blockquote_renders_inline_styles — TestBackend buffer shows styled text inside blockquote.
    #[test]
    fn blockquote_renders_inline_styles() {
        let width = 60u16;
        let height = 6u16;
        // Blockquote with bold and italic text
        let element = Element::agent(
            "> **bold** quote
> and *italic* too",
        )
        .at(0.0);
        let rendered = to_lines_internal(&element, width);

        // Blockquote should render at least one line with the bar character
        assert!(
            !rendered.is_empty(),
            "blockquote should produce at least one line"
        );

        // Check that blockquote character appears
        let has_bar = rendered
            .iter()
            .any(|line| line.spans.iter().any(|s| s.content.contains('│')));
        assert!(
            has_bar,
            "blockquote should have │ character: {:?}",
            rendered
        );

        // Render to terminal and check buffer
        let snap = Snapshot {
            elements: Arc::new([element]),
            line_counts: Arc::new([rendered.len()]),
            total_lines: rendered.len(),
            last_visible_height: height,
            content_width: width,
            ..Default::default()
        };

        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| render_messages(f, &snap, f.area()))
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();
        // Blockquote should appear in output
        assert!(
            content.contains('│'),
            "buffer should contain │ for blockquote: {}",
            content
        );
    }
}
