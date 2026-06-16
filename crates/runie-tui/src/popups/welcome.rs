//! Welcome / launcher screen — shown when no session is active.

use std::sync::Arc;

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use runie_core::Snapshot;

use crate::popups::palette_popup_rect;
use crate::theme::{block_popup, color_accent, color_bg_panel, color_dim};

/// Render the welcome/launcher overlay covering the main area.
pub fn render_welcome(f: &mut Frame, snap: &Snapshot) {
    let area = palette_popup_rect(f.area());
    f.buffer_mut()
        .set_style(area, Style::default().bg(color_bg_panel()));

    let block = block_popup("Runie");
    let inner = block.inner(area);
    f.render_widget(Paragraph::new("").block(block), area);

    let content = build_welcome_content(snap, inner);
    let para = Paragraph::new(content);
    f.render_widget(para, inner);
}

fn build_welcome_content(snap: &Snapshot, inner: Rect) -> Vec<Line<'static>> {
    use ratatui::style::Color;
    let mut lines = Vec::new();

    // ── Header ──────────────────────────────────────────────────────────────
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("Runie", Style::default().fg(color_accent()).bold()),
    ]));
    lines.push(Line::from(""));

    // ── Session options ─────────────────────────────────────────────────────
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("New session", Style::default().fg(Color::White)),
        Span::raw("     "),
        Span::styled("Ctrl+N", Style::default().fg(color_accent())),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("Resume session", Style::default().fg(Color::White)),
        Span::raw("  "),
        Span::styled("Ctrl+R", Style::default().fg(color_accent())),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("Command palette", Style::default().fg(Color::White)),
        Span::raw(" "),
        Span::styled("Ctrl+P", Style::default().fg(color_accent())),
    ]));
    lines.push(Line::from(""));

    // ── Quit ────────────────────────────────────────────────────────────────
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("Quit", Style::default().fg(Color::White)),
        Span::raw("         "),
        Span::styled("Ctrl+Q", Style::default().fg(color_accent())),
    ]));
    lines.push(Line::from(""));

    // ── Recent sessions ─────────────────────────────────────────────────────
    if !snap.session_tree_items.is_empty() {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Recent sessions", Style::default().fg(color_dim()).bold()),
        ]));
        for (depth, preview) in snap.session_tree_items.iter().take(5) {
            let indent = "  ".repeat(*depth + 1);
            lines.push(Line::from(vec![
                Span::raw(indent),
                Span::styled(
                    preview.chars().take(40).collect::<String>(),
                    Style::default().fg(color_dim()),
                ),
            ]));
        }
        lines.push(Line::from(""));
    }

    // ── Hint at bottom ──────────────────────────────────────────────────────
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("Tab", Style::default().fg(color_accent())),
        Span::raw(" to focus the input prompt"),
    ]));

    // Pad to fill the popup height
    let used = lines.len() as u16;
    let remaining = inner.height.saturating_sub(used);
    for _ in 0..remaining {
        lines.push(Line::from(""));
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};
    use runie_core::Snapshot;

    #[test]
    fn welcome_renders_new_resume_quit() {
        let snap = Snapshot::default();
        let backend = TestBackend::new(60, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render_welcome(f, &snap)).unwrap();
        let buf = terminal.backend().buffer();
        let content: String = buf
            .content
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(content.contains("New session"), "should show New session: {content}");
        assert!(content.contains("Resume session"), "should show Resume session: {content}");
        assert!(content.contains("Quit"), "should show Quit: {content}");
        assert!(content.contains("Ctrl+N"), "should show Ctrl+N: {content}");
        assert!(content.contains("Ctrl+R"), "should show Ctrl+R: {content}");
        assert!(content.contains("Ctrl+Q"), "should show Ctrl+Q: {content}");
    }

    #[test]
    fn welcome_renders_with_recent_sessions() {
        let snap = Snapshot {
            session_tree_items: Arc::new([
                (0usize, "Implement X feature".to_string()),
                (1usize, "Fix bug Y".to_string()),
            ]),
            ..Default::default()
        };
        let backend = TestBackend::new(60, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render_welcome(f, &snap)).unwrap();
        let buf = terminal.backend().buffer();
        let content: String = buf
            .content
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(
            content.contains("Implement X feature"),
            "should show recent session: {content}"
        );
        assert!(
            content.contains("Recent sessions"),
            "should show Recent sessions header: {content}"
        );
    }
}
