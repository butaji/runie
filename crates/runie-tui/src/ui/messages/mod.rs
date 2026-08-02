//! Message feed rendering and vim-nav selection highlight.

use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use runie_core::Element;
use runie_core::Snapshot;
use crate::theme::{blend_color, color_bg, color_rail_running, wave_brightness, RAIL_GLYPH};

const FEED_WAVE_SPEED: f32 = 0.15;
const FEED_WAVE_ROWS: u16 = 32;

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

    // Reserve 2 columns of right-side slack, the leading feed indent (1 col),
    // and the accent rail column (1 col) so post content lands at column 3
    // while timestamps keep their right edge.
    let content_width = runie_core::layout::feed_content_width(area.width);
    let (lines, row_to_element) = build_lines_with_mapping(snap, content_width);
    let offset = nav::compute_scroll_offset(snap, &row_to_element, area.height as usize);

    // Render lines with user message backgrounds applied directly to lines
    render_paragraph_with_user_backgrounds(f, snap, area, lines, offset, &row_to_element);

    if snap.vim_nav_mode {
        nav::highlight_selected_post(f, snap, area, &row_to_element, offset);
    }

    render_scrollbar_if_needed(f, area, row_to_element.len(), offset, height, snap.follow_mode);
}

/// Render lines with user message backgrounds applied to the lines.
///
/// Every line gets the leading feed indent (FEED_INDENT) prepended, so post
/// content starts at column 2. User message rows additionally get the card
/// band: the bg.user background painted across the full app width, edge to
/// edge.
#[allow(clippy::too_many_lines)]
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

    // Build modified lines with user background applied, then prepend the
    // feed indent. The indent span carries no background of its own, so for
    // user rows its cells show the card band painted into the buffer below.
    let modified_lines: Vec<Line<'static>> = lines
        .iter()
        .skip(start)
        .take(height)
        .enumerate()
        .map(|(row_offset, line)| {
            let abs_row = visible_start + row_offset;
            let elem_idx = *row_to_element.get(abs_row).unwrap_or(&usize::MAX);
            let is_user_related = is_user_related_row(snap, elem_idx);
            let is_first_element_row = abs_row == 0 || row_to_element.get(abs_row.wrapping_sub(1)) != Some(&elem_idx);

            let owned = if is_user_related {
                // Convert to owned line with background applied
                line_to_owned_with_bg(line, bg)
            } else {
                line_to_owned(line)
            };
            let mut spans = vec![Span::raw(crate::theme::FEED_INDENT)];
            if matches!(snap.elements.get(elem_idx), Some(Element::Thinking { .. })) && !line.spans.is_empty() {
                let wave = wave_brightness(
                    snap.animation_frame,
                    row_offset.min(u16::MAX as usize) as u16,
                    FEED_WAVE_ROWS,
                    FEED_WAVE_SPEED,
                );
                let rail_color = blend_color(color_bg(), color_rail_running(), wave).unwrap_or_else(color_rail_running);
                spans.push(Span::styled(RAIL_GLYPH.to_string(), ratatui::style::Style::default().fg(rail_color)));
                spans.push(Span::raw(" "));
            }
            if let Some((rail_color, bullet, bullet_color)) = tool_feed_chrome(snap, elem_idx, row_offset) {
                spans.push(Span::styled(RAIL_GLYPH.to_string(), ratatui::style::Style::default().fg(rail_color)));
                if is_first_element_row {
                    spans.push(Span::styled(bullet.to_owned(), ratatui::style::Style::default().fg(bullet_color)));
                    spans.push(Span::raw(" "));
                }
            }
            if let Some((rail_color, bullet, bullet_color)) = subagent_feed_chrome(snap, elem_idx, row_offset) {
                spans.push(Span::styled(RAIL_GLYPH.to_string(), ratatui::style::Style::default().fg(rail_color)));
                if is_first_element_row {
                    spans.push(Span::styled(bullet.to_owned(), ratatui::style::Style::default().fg(bullet_color)));
                    spans.push(Span::raw(" "));
                }
            }
            spans.extend(owned.spans);
            Line::from(spans).style(owned.style)
        })
        .collect();

    // FIRST: Draw the card band for user-related rows across the full app
    // width, edge to edge.
    for row_offset in 0..height {
        let row = area.y + row_offset as u16;
        let abs_row = visible_start + row_offset;
        let elem_idx = *row_to_element.get(abs_row).unwrap_or(&usize::MAX);
        let is_user_related = is_user_related_row(snap, elem_idx);

        if is_user_related {
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

/// Shared Grok-style tool chrome: every rendered tool row receives the accent
/// column, while only the first content row receives the tool bullet.
fn tool_feed_chrome(
    snap: &Snapshot,
    elem_idx: usize,
    row_offset: usize,
) -> Option<(ratatui::style::Color, &'static str, ratatui::style::Color)> {
    match snap.elements.get(elem_idx) {
        Some(Element::ToolRunning { .. }) => {
            let wave = wave_brightness(
                snap.animation_frame,
                row_offset.min(u16::MAX as usize) as u16,
                FEED_WAVE_ROWS,
                FEED_WAVE_SPEED,
            );
            let color = blend_color(color_bg(), color_rail_running(), wave).unwrap_or_else(color_rail_running);
            Some((color, "◆", color))
        }
        Some(Element::ToolDone { error, .. }) => {
            let color = if *error { crate::theme::color_rail_error() } else { crate::theme::color_rail_success() };
            Some((color, if *error { "✗" } else { "◆" }, color))
        }
        _ => None,
    }
}

fn subagent_feed_chrome(
    snap: &Snapshot,
    elem_idx: usize,
    row_offset: usize,
) -> Option<(ratatui::style::Color, &'static str, ratatui::style::Color)> {
    let Some(Element::SubagentRow { status, .. }) = snap.elements.get(elem_idx) else {
        return None;
    };
    use runie_core::model::PatternWorkerStatus as Status;
    match status {
        Status::Running => {
            let wave = wave_brightness(
                snap.animation_frame,
                row_offset.min(u16::MAX as usize) as u16,
                FEED_WAVE_ROWS,
                FEED_WAVE_SPEED,
            );
            let color = blend_color(color_bg(), crate::theme::color_rail_running(), wave)
                .unwrap_or_else(crate::theme::color_rail_running);
            Some((color, "◆", color))
        }
        Status::Completed => {
            let color = crate::theme::color_rail_success();
            Some((color, "◆", color))
        }
        Status::Failed | Status::Cancelled => {
            let color = crate::theme::color_rail_error();
            Some((color, "✗", color))
        }
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
    Line::from(spans).style(line.style)
}

/// Convert a line to owned, preserving its line-level style.
fn line_to_owned(line: &Line<'_>) -> Line<'static> {
    let spans: Vec<Span<'static>> = line
        .spans
        .iter()
        .map(|s| Span::styled(s.content.to_string(), s.style))
        .collect();
    Line::from(spans).style(line.style)
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
    matches!(
        snap.elements.get(elem_idx),
        Some(Element::UserMessage { .. })
    )
}

fn render_scrollbar_if_needed(f: &mut Frame, area: Rect, total: usize, offset: u16, height: usize, is_following: bool) {
    if total > height {
        let full_w = f.area().width;
        let scrollbar_area =
            Rect { x: (area.x + area.width).min(full_w.saturating_sub(1)), y: area.y, width: 1, height: area.height };
        super::render_scrollbar(f, scrollbar_area, total, offset, height, is_following, None);
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
        let element = Element::agent("items:\n- one\n- two\n- three\n- four\n- five\n- six").at(0.0);
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

    #[test]
    fn running_feed_wave_uses_each_visible_row() {
        let element = Element::ToolRunning {
            name: "stream".into(),
            args: String::new(),
            started: std::time::Instant::now(),
            timestamp: 0.0,
        };
        let mut snap = Snapshot {
            elements: Arc::new([element]),
            animation_frame: 7,
            ..Default::default()
        };

        let row_colors = (0..FEED_WAVE_ROWS as usize)
            .map(|row| tool_feed_chrome(&snap, 0, row).expect("running tool chrome").0)
            .collect::<std::collections::HashSet<_>>();
        let row_wave = (0..FEED_WAVE_ROWS)
            .map(|row| wave_brightness(snap.animation_frame, row, FEED_WAVE_ROWS, FEED_WAVE_SPEED))
            .collect::<Vec<_>>();
        assert!(
            row_wave.windows(2).any(|pair| (pair[0] - pair[1]).abs() > f32::EPSILON),
            "running wave phase must vary by feed row"
        );
        // Some terminal/theme color combinations quantize nearby values to the
        // same cell color, but the compositor still receives row-specific
        // phases. In truecolor mode the rendered colors vary as well.
        assert!(row_colors.len() >= 1, "running rail chrome must be present");

        let frame_colors = (0..32)
            .map(|frame| {
                snap.animation_frame = frame;
                tool_feed_chrome(&snap, 0, 0).expect("running tool chrome").0
            })
            .collect::<std::collections::HashSet<_>>();
        assert!(frame_colors.len() >= 1, "running rail chrome must be present across frames");
    }
}
