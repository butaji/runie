//! Right-aligned user message bubble helpers.

use ratatui::{
    style::Style,
    text::{Line, Span},
};

use crate::markdown_render::{md_to_spans, MdSpan};
use crate::theme::color_fg_bright;

use super::span_width;

/// Compute the bubble width: text width plus horizontal padding, capped at the
/// available inner width so the right margin is always preserved.
pub fn compute_width(rows: &[Vec<crate::markdown_render::MdSpan>], inner_width: u16) -> u16 {
    let pad = super::BUBBLE_H_PAD * 2;
    let max_text = rows
        .iter()
        .map(|r| span_width(&md_to_spans(r)))
        .max()
        .unwrap_or(0);
    (max_text + pad).min(inner_width)
}

/// Top/bottom bubble line: left fill (feed background) + bubble fill (user bg).
pub fn margin_line(
    left_fill: u16,
    bubble_width: u16,
    content_width: u16,
    bg: ratatui::style::Color,
) -> Line<'static> {
    let mut spans = vec![Span::raw(" ".repeat(left_fill as usize))];
    spans.push(Span::styled(
        " ".repeat(bubble_width as usize),
        Style::default().bg(bg),
    ));
    let right_fill = content_width.saturating_sub(left_fill).saturating_sub(bubble_width);
    if right_fill > 0 {
        spans.push(Span::raw(" ".repeat(right_fill as usize)));
    }
    Line::from(spans)
}

/// Content line inside the bubble: left fill, padding, text, right padding.
pub fn content_line(
    row: &[MdSpan],
    left_fill: u16,
    bubble_width: u16,
    h_pad: u16,
    bg: ratatui::style::Color,
) -> Line<'static> {
    let text_style = Style::default().fg(color_fg_bright()).bg(bg);
    let bg_style = Style::default().bg(bg);

    let text_spans = md_to_spans(row);
    let text_width = span_width(&text_spans);
    let right_pad = bubble_width.saturating_sub(text_width).saturating_sub(h_pad * 2);

    let mut spans = vec![Span::raw(" ".repeat(left_fill as usize))];
    spans.push(Span::styled(" ".repeat(h_pad as usize), bg_style));
    for s in text_spans {
        spans.push(Span::styled(s.content.to_string(), text_style));
    }
    if right_pad > 0 {
        spans.push(Span::styled(" ".repeat(right_pad as usize), bg_style));
    }
    spans.push(Span::styled(" ".repeat(h_pad as usize), bg_style));

    Line::from(spans)
}
