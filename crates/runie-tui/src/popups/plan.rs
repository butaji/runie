//! Plan mode overlay — shown when plan mode is active.
//!
//! Displays the current plan markdown and indicates that write tools are blocked.

use ratatui::{layout::Rect, style::Style, text::Line, widgets::Paragraph, Frame};
use runie_core::Snapshot;

use crate::popups::panel::{hotkey_area, hotkey_area_height, setup_popup};
use crate::theme::{color_accent, color_bg_panel, style_hint};

/// Render the plan mode overlay if plan mode is active.
pub fn render_plan_panel(f: &mut Frame, snap: &Snapshot) {
    if !snap.plan_mode {
        return;
    }

    let inner = setup_popup(f, " Plan Mode ");
    let bg = color_bg_panel();

    let header_height = 2u16;
    let content_height = inner.height.saturating_sub(header_height + hotkey_area_height());

    let header_area = Rect { x: inner.x, y: inner.y, width: inner.width, height: header_height };
    let content_area = Rect {
        x: inner.x,
        y: inner.y + header_height,
        width: inner.width,
        height: content_height,
    };

    let mut header_lines = Vec::new();
    header_lines.push(Line::from(vec![
        ratatui::text::Span::styled("✦ ", Style::default().fg(color_accent())),
        ratatui::text::Span::raw("Plan mode active — write tools blocked"),
    ]));
    header_lines.push(Line::from(""));
    f.render_widget(Paragraph::new(header_lines).style(Style::default().bg(bg)), header_area);

    let content_lines = build_plan_lines(snap);
    f.render_widget(Paragraph::new(content_lines).style(Style::default().bg(bg)), content_area);

    let footer_line = Line::from("[Enter] Approve plan   [Esc] /plan off").style(style_hint());
    f.render_widget(Paragraph::new(footer_line), hotkey_area(&inner));
}

fn build_plan_lines(snap: &Snapshot) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let content = &snap.active_plan_content;

    if content.is_empty() {
        lines.push(Line::from("No plan content yet.").style(style_hint()));
    } else {
        for (i, line) in content.lines().enumerate() {
            if i >= 12 {
                lines.push(Line::from(format!(
                    "... ({} more lines)",
                    content.lines().count().saturating_sub(12)
                )));
                break;
            }
            lines.push(Line::from(line.to_string()));
        }
    }

    lines.push(Line::from(""));
    lines
}
