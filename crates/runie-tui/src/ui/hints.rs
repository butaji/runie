//! Hint bar and transient message rendering.

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::theme::{
    color_bg, color_bg_panel, color_error, color_success, color_warning, style_hint, style_hint_key,
};

pub(crate) fn hints(f: &mut Frame, snap: &runie_core::Snapshot, area: Rect) {
    if let Some(ref msg) = snap.transient_message {
        render_transient(f, snap, area, msg);
    } else {
        let line = Line::from(parse_hint_spans(&snap.hint_text));
        f.render_widget(Paragraph::new(line), area);
    }
}

fn render_transient(f: &mut Frame, snap: &runie_core::Snapshot, area: Rect, msg: &str) {
    let (label, bg) = transient_style(snap);
    let badge_bg = crate::theme::darken(bg, 0.8);
    let margin_bg = crate::theme::darken(bg, 0.85);
    let dark_text = color_bg();
    let margin_style = Style::default().fg(dark_text).bg(margin_bg);
    let msg_style = Style::default().fg(dark_text).bg(bg);
    let badge_style = Style::default().fg(dark_text).bg(badge_bg).bold();
    let content_len = label.len() + 2 + msg.len();
    let fill_len = (area.width as usize).saturating_sub(content_len + 1);
    let fill = " ".repeat(fill_len.max(1));
    let spans = vec![
        Span::styled(" ", margin_style),
        Span::styled(label, badge_style),
        Span::styled(" ", margin_style),
        Span::styled(format!(" {}", msg), msg_style),
        Span::styled(&fill, msg_style),
    ];
    let block = Block::default().borders(Borders::NONE).style(margin_style);
    f.render_widget(Paragraph::new(Line::from(spans)).block(block), area);
}

fn transient_style(snap: &runie_core::Snapshot) -> (&'static str, ratatui::style::Color) {
    match snap.transient_level {
        Some(runie_core::event::TransientLevel::Success) => ("\\ok\\", color_success()),
        Some(runie_core::event::TransientLevel::Warning) => ("\\warn\\", color_warning()),
        Some(runie_core::event::TransientLevel::Error) => ("\\err\\", color_error()),
        _ => ("", color_bg_panel()),
    }
}

pub(crate) fn parse_hint_spans(text: &str) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let segments: Vec<&str> = text.split(" · ").collect();
    for (i, segment) in segments.iter().enumerate() {
        if let Some(space_idx) = segment.find(' ') {
            let key = &segment[..space_idx];
            let desc = &segment[space_idx..];
            spans.push(Span::styled(key.to_owned(), style_hint_key()));
            spans.push(Span::styled(desc.to_owned(), style_hint()));
        } else {
            spans.push(Span::styled(segment.to_string(), style_hint()));
        }
        if i + 1 < segments.len() {
            spans.push(Span::styled(" · ".to_owned(), style_hint()));
        }
    }
    spans
}
