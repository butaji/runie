//! Plan mode overlay — shown when plan mode is active.
//!
//! Displays the current plan markdown and indicates that write tools are blocked.

use ratatui::{
    layout::Rect,
    prelude::Text,
    style::Style,
    text::Line,
    Frame,
};
use runie_core::Snapshot;
use tui_popup::Popup;

use crate::popups::palette_popup_rect;
use crate::theme::{color_accent, color_bg_panel, style_hint};

/// Render the plan mode overlay if plan mode is active.
pub fn render_plan_panel(f: &mut Frame, snap: &Snapshot) {
    if !snap.plan_mode {
        return;
    }

    let area = palette_popup_rect(f.area());
    let bg = color_bg_panel();
    let lines = build_plan_lines(snap);

    let content = Text::from(lines).style(Style::default().bg(bg));
    let popup = Popup::new(content)
        .title(" Plan Mode ")
        .style(Style::default().bg(bg));
    f.render_widget(popup, area);

    // Explicitly set inner background (tui-popup uses Clear which resets to terminal bg).
    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };
    f.buffer_mut().set_style(inner, Style::default().bg(bg));
}

fn build_plan_lines(snap: &Snapshot) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    // Header with status
    lines.push(Line::from(vec![
        ratatui::text::Span::styled("✦ ", Style::default().fg(color_accent())),
        ratatui::text::Span::raw("Plan mode active — write tools blocked"),
    ]));
    lines.push(Line::from(""));

    // Plan content (truncated if too long)
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
    lines.push(Line::from("[Enter] Approve plan   [Esc] /plan off").style(style_hint()));

    lines
}
