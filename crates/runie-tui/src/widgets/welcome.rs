//! Deterministic full-mode welcome surface.

use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

/// The full-mode welcome surface shown before the first prompt is submitted.
#[derive(Debug, Clone, Copy, Default)]
pub struct WelcomeWidget;

impl Widget for WelcomeWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 24 || area.height < 8 {
            return;
        }
        if area.height < 16 {
            let surface = Rect {
                x: area.x
                    + area
                        .width
                        .saturating_sub(42.min(area.width.saturating_sub(4)))
                        / 2,
                y: area.y + area.height.saturating_sub(10) / 2,
                width: 42.min(area.width.saturating_sub(4)),
                height: 10,
            };
            let lines = vec![
                Line::from(Span::styled(
                    "Runie",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from("v0.1.0"),
                Line::from("Model · runie-core"),
                Line::from(""),
                Line::from("New session"),
                Line::from("/help for commands"),
                Line::from("Ctrl+Q · quit"),
                Line::from("◆ session_start"),
            ];
            Paragraph::new(lines)
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL).title(" welcome "))
                .render(surface, buf);
            return;
        }
        let bold = Style::default().add_modifier(Modifier::BOLD);
        let x = area.x.saturating_add(13);
        let action = |label: &str, shortcut: &str| {
            Line::from(vec![
                Span::styled(label.to_owned(), bold),
                Span::raw(" ".repeat(45usize.saturating_sub(label.len()))),
                Span::raw(shortcut.to_owned()),
            ])
        };
        Paragraph::new(vec![
            action("New worktree", "ctrl+w"),
            action("Resume session", "ctrl+s"),
            Line::from(Span::styled("Changelog", bold)),
            action("Quit", "ctrl+q"),
        ])
        .render(
            Rect::new(x, area.y + 3, area.width.saturating_sub(13), 4),
            buf,
        );
        Paragraph::new(vec![
            Line::from(Span::styled("Workflows are here!", bold)),
            Line::from("Try them out using /workflows."),
        ])
        .render(
            Rect::new(x, area.y + 8, area.width.saturating_sub(13), 2),
            buf,
        );
        Paragraph::new(Line::from(vec![
            Span::styled("Tip: ", bold),
            Span::raw("Use Ctrl+Enter to interject messages. Or just Enter to queue messages."),
        ]))
        .render(Rect::new(area.x, area.y + 14, area.width, 1), buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn welcome_surface_snapshot() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 80, 24));
        WelcomeWidget.render(Rect::new(0, 0, 80, 24), &mut buffer);
        let text = (0..24)
            .map(|y| {
                (0..80)
                    .map(|x| buffer.cell((x, y)).expect("cell").symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        insta::assert_snapshot!("welcome-full-mode", text);
    }
}
