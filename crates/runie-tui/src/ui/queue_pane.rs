//! Queue pane rendering (grok parity — queued messages above the input).
//!
//! Rows: `#N first-line` with a gray `(+N lines)` suffix for multiline
//! messages. The selected row (when the pane is focused) carries a highlight
//! background.

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::theme::{color_bg_panel, color_dim, color_user_text, color_warning};
use runie_core::Snapshot;

/// Max queue rows rendered (grok parity `MAX_QUEUE_HEIGHT`).
pub const MAX_QUEUE_HEIGHT: u16 = 3;

/// Height of the queue pane: 0 when hidden/empty, else clamped to 1..=3.
pub fn queue_pane_height(snap: &Snapshot) -> u16 {
    if !snap.queue_pane_visible || snap.queued_messages.is_empty() {
        return 0;
    }
    snap.queued_messages.len().min(MAX_QUEUE_HEIGHT as usize) as u16
}

/// Render the queue pane rows (nothing when hidden or empty).
pub fn render(f: &mut Frame, snap: &Snapshot, area: Rect) {
    if area.height == 0 || snap.queued_messages.is_empty() {
        return;
    }
    let selected = snap.queue_pane_selected.min(snap.queued_messages.len() - 1);
    let focused = snap.queue_pane_focused;

    for (row, msg) in snap.queued_messages.iter().take(area.height as usize).enumerate() {
        let y = area.y + row as u16;
        let row_area = Rect::new(area.x, y, area.width, 1);
        let is_selected = focused && row == selected;

        let prefix = format!("#{} ", msg.position);
        let mut content = msg.first_line.clone();
        if msg.line_count > 1 {
            content.push_str(&format!(" (+{} lines)", msg.line_count - 1));
        }

        let body_style = match msg.kind {
            runie_core::model::QueuedMessageKind::FollowUp => Style::new().fg(color_user_text()),
            runie_core::model::QueuedMessageKind::Steering => Style::new().fg(color_warning()),
        };
        let line = Line::from(vec![
            Span::styled(prefix, Style::new().fg(color_dim())),
            Span::styled(content, body_style),
        ]);

        let style = if is_selected {
            Style::new().bg(color_bg_panel())
        } else {
            Style::new()
        };
        f.render_widget(Paragraph::new(line).style(style), row_area);
    }
}
