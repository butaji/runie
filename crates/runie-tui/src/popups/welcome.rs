//! Welcome / launcher screen — shown when no session is active.

#![allow(clippy::vec_init_then_push)]

#[cfg(test)]
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
use crate::Stylize;

/// Render the welcome/launcher overlay covering the main area.
#[allow(clippy::vec_init_then_push)]
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
    let mut lines = Vec::new();
    build_header(&mut lines);
    build_session_options(&mut lines);
    build_quit_option(&mut lines);
    build_recent_sessions(&mut lines, snap);
    build_hint(&mut lines);
    pad_to_height(&mut lines, inner.height);
    lines
}

fn build_header(lines: &mut Vec<Line<'static>>) {
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("Runie", Style::default().fg(color_accent()).bold()),
    ]));
    lines.push(Line::from(""));
}

fn option_line(prefix: &str, label: &str, spacing: &str, key: &str) -> Line<'static> {
    Line::from(vec![
        Span::raw(prefix.to_string()),
        label.to_string().white(),
        Span::raw(spacing.to_string()),
        Span::styled(key.to_string(), Style::default().fg(color_accent())),
    ])
}

fn build_session_options(lines: &mut Vec<Line<'static>>) {
    lines.push(option_line("  ", "New session", "     ", "Ctrl+N"));
    lines.push(option_line("  ", "Resume session", "  ", "Ctrl+R"));
    lines.push(option_line("  ", "Command palette", " ", "Ctrl+P"));
    lines.push(Line::from(""));
}

fn build_quit_option(lines: &mut Vec<Line<'static>>) {
    lines.push(option_line("  ", "Quit", "         ", "Ctrl+Q"));
    lines.push(Line::from(""));
}

fn build_recent_sessions(lines: &mut Vec<Line<'static>>, snap: &Snapshot) {
    if snap.session_tree_items.is_empty() {
        return;
    }
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

fn build_hint(lines: &mut Vec<Line<'static>>) {
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("Tab", Style::default().fg(color_accent())),
        Span::raw(" to focus the input prompt"),
    ]));
}

fn pad_to_height(lines: &mut Vec<Line<'static>>, height: u16) {
    let used = lines.len() as u16;
    let remaining = height.saturating_sub(used);
    for _ in 0..remaining {
        lines.push(Line::from(""));
    }
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
