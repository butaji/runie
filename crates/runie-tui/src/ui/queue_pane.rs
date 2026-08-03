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

use crate::theme::{color_bg_panel, color_dim, color_error, color_fg, color_user_text, color_warning};
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
    if area.height == 0 || !snap.queue_pane_visible || snap.queued_messages.is_empty() {
        return;
    }
    let selected = snap.queue_pane_selected.min(snap.queued_messages.len() - 1);
    let focused = snap.queue_pane_focused;
    let visible_rows = area.height.min(MAX_QUEUE_HEIGHT);
    let start = queue_visible_start(selected, snap.queued_messages.len(), visible_rows as usize);

    for (row, msg) in snap
        .queued_messages
        .iter()
        .skip(start)
        .take(visible_rows as usize)
        .enumerate()
    {
        let y = area.y + row as u16;
        let row_area = Rect::new(area.x, y, area.width, 1);
        let is_selected = focused && start + row == selected;

        let prefix = format!("#{} ", msg.position);
        let mut content = msg.first_line.clone();
        if msg.line_count > 1 {
            content.push_str(&format!(" (+{} lines)", msg.line_count - 1));
        }

        let actions = if is_selected {
            if snap.turn_active {
                " [Send now][cancel]"
            } else {
                " [cancel]"
            }
        } else {
            ""
        };
        let prefix_width = prefix.chars().count();
        let action_width = actions.chars().count();
        let content_width = (area.width as usize)
            .saturating_sub(prefix_width + action_width)
            .max(1);
        content = truncate_preserving_suffix(&content, content_width);

        let body_style = match msg.kind {
            runie_core::model::QueuedMessageKind::FollowUp => Style::new().fg(color_user_text()),
            runie_core::model::QueuedMessageKind::Steering => Style::new().fg(color_warning()),
        };
        let mut spans = vec![Span::styled(prefix, Style::new().fg(color_dim())), Span::styled(content, body_style)];
        if is_selected {
            if snap.turn_active {
                spans.push(Span::styled(" [Send now]", Style::new().fg(color_fg())));
            }
            spans.push(Span::styled("[cancel]", Style::new().fg(color_error())));
        }
        let line = Line::from(spans);

        let style = if is_selected {
            Style::new().bg(color_bg_panel())
        } else {
            Style::new()
        };
        f.render_widget(Paragraph::new(line).style(style), row_area);
    }
}

fn queue_visible_start(selected: usize, total: usize, height: usize) -> usize {
    if height == 0 || total <= height {
        return 0;
    }
    selected
        .saturating_sub(height - 1)
        .min(total.saturating_sub(height))
}

/// Keep the multiline suffix visible when a queue row is narrow. This uses
/// terminal cells approximately by character count; the queue preview is
/// intentionally ASCII/emoji-light and the surrounding UI handles wide glyphs.
fn truncate_preserving_suffix(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_owned();
    }
    let suffix_start = text.rfind(" (+").unwrap_or(text.len());
    let suffix = &text[suffix_start..];
    let suffix_len = suffix.chars().count();
    if suffix_start < text.len() && suffix_len < width {
        let head: String = text[..suffix_start]
            .chars()
            .take(width - suffix_len - 1)
            .collect();
        return format!("{}…{}", head, suffix);
    }
    let head: String = text.chars().take(width.saturating_sub(1)).collect();
    format!("{}…", head)
}

#[cfg(test)]
mod tests {
    use super::{queue_visible_start, truncate_preserving_suffix};

    #[test]
    fn overflow_keeps_selected_row_visible() {
        assert_eq!(queue_visible_start(0, 5, 3), 0);
        assert_eq!(queue_visible_start(2, 5, 3), 0);
        assert_eq!(queue_visible_start(3, 5, 3), 1);
        assert_eq!(queue_visible_start(4, 5, 3), 2);
    }

    #[test]
    fn truncation_preserves_multiline_suffix() {
        assert_eq!(
            truncate_preserving_suffix("a long prompt (+3 lines)", 15),
            "a l… (+3 lines)"
        );
    }

    #[test]
    fn truncation_handles_plain_text_and_tiny_widths() {
        assert_eq!(truncate_preserving_suffix("short", 10), "short");
        assert_eq!(truncate_preserving_suffix("abcdef", 1), "…");
    }
}
