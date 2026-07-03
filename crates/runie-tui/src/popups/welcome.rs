//! Welcome / launcher screen — shown when no session is active.

// allow: vec![] then push is necessary — welcome content is built dynamically
//       by helper functions that append to the lines Vec
#![allow(clippy::vec_init_then_push)]

#[cfg(test)]
use std::sync::Arc;

use ratatui::{
    layout::Rect,
    prelude::Text,
    style::Style,
    text::{Line, Span},
    Frame,
};
use runie_core::Snapshot;
use tui_popup::Popup;

use crate::popups::palette_popup_rect;
use crate::theme::{color_accent, color_bg_panel, color_dim};
use crate::Stylize;

/// Render the welcome/launcher overlay covering the main area.
// allow: vec![] then push is necessary — initial content is not statically known
#[allow(clippy::vec_init_then_push)]
pub fn render_welcome(f: &mut Frame, snap: &Snapshot) {
    let area = palette_popup_rect(f.area());
    let bg = color_bg_panel();

    // Build welcome content lines.
    let lines = build_welcome_content(snap);

    // Use tui-popup for the shell (border + title + centering).
    let content = Text::from(lines).style(Style::default().bg(bg));
    let popup = Popup::new(content)
        .title("Runie")
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

fn build_welcome_content(snap: &Snapshot) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    build_header(&mut lines);
    build_session_options(&mut lines);
    build_quit_option(&mut lines);
    build_recent_sessions(&mut lines, snap);
    build_hint(&mut lines);
    pad_to_height(&mut lines);
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
        Span::raw(prefix.to_owned()),
        label.to_owned().white(),
        Span::raw(spacing.to_owned()),
        Span::styled(key.to_owned(), Style::default().fg(color_accent())),
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

fn pad_to_height(_lines: &mut Vec<Line<'static>>) {
    // Content height is determined by tui-popup's auto-sizing; no padding needed.
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};
    use runie_core::Snapshot;

    #[test]
    fn welcome_renders_new_resume_quit() {
        let snap = Snapshot::default();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render_welcome(f, &snap)).unwrap();
        let buf = terminal.backend().buffer();
        let content: String = buf.content.iter().map(|c| c.symbol()).collect();
        assert!(
            content.contains("New session"),
            "should show New session: {content}"
        );
        assert!(
            content.contains("Resume session"),
            "should show Resume session: {content}"
        );
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
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render_welcome(f, &snap)).unwrap();
        let buf = terminal.backend().buffer();
        let content: String = buf.content.iter().map(|c| c.symbol()).collect();
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
