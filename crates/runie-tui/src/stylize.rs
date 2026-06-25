//! Convenience extension trait for styling strings.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

/// Extension trait for fluent string styling.
pub trait Stylize {
    /// Style the text red.
    fn red(self) -> Span<'static>;
    /// Style the text green.
    fn green(self) -> Span<'static>;
    /// Style the text blue.
    fn blue(self) -> Span<'static>;
    /// Style the text white.
    fn white(self) -> Span<'static>;
    /// Style the text dim.
    fn dim(self) -> Span<'static>;
    /// Style the text bold.
    fn bold(self) -> Span<'static>;
    /// Style the text underlined.
    fn underlined(self) -> Span<'static>;
}

impl Stylize for &str {
    fn red(self) -> Span<'static> {
        Span::styled(self.to_owned(), Style::default().fg(Color::Red))
    }
    fn green(self) -> Span<'static> {
        Span::styled(self.to_owned(), Style::default().fg(Color::Green))
    }
    fn blue(self) -> Span<'static> {
        Span::styled(self.to_owned(), Style::default().fg(Color::Blue))
    }
    fn white(self) -> Span<'static> {
        Span::styled(self.to_owned(), Style::default().fg(Color::White))
    }
    fn dim(self) -> Span<'static> {
        Span::styled(
            self.to_owned(),
            Style::default().add_modifier(Modifier::DIM),
        )
    }
    fn bold(self) -> Span<'static> {
        Span::styled(
            self.to_owned(),
            Style::default().add_modifier(Modifier::BOLD),
        )
    }
    fn underlined(self) -> Span<'static> {
        Span::styled(
            self.to_owned(),
            Style::default().add_modifier(Modifier::UNDERLINED),
        )
    }
}

impl Stylize for String {
    fn red(self) -> Span<'static> {
        Span::styled(self, Style::default().fg(Color::Red))
    }
    fn green(self) -> Span<'static> {
        Span::styled(self, Style::default().fg(Color::Green))
    }
    fn blue(self) -> Span<'static> {
        Span::styled(self, Style::default().fg(Color::Blue))
    }
    fn white(self) -> Span<'static> {
        Span::styled(self, Style::default().fg(Color::White))
    }
    fn dim(self) -> Span<'static> {
        Span::styled(self, Style::default().add_modifier(Modifier::DIM))
    }
    fn bold(self) -> Span<'static> {
        Span::styled(self, Style::default().add_modifier(Modifier::BOLD))
    }
    fn underlined(self) -> Span<'static> {
        Span::styled(self, Style::default().add_modifier(Modifier::UNDERLINED))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    #[test]
    fn stylize_red_returns_red_styled() {
        let span = "error".red();
        assert_eq!(span.style.fg, Some(Color::Red));
    }

    #[test]
    fn stylize_green_on_string() {
        let span = String::from("ok").green();
        assert_eq!(span.style.fg, Some(Color::Green));
    }

    #[test]
    fn stylize_chain_works() {
        // Chaining requires taking the styled span and applying modifiers.
        let span = "warn".red();
        let span = Span::styled(span.content, span.style.add_modifier(Modifier::BOLD));
        assert_eq!(span.style.fg, Some(Color::Red));
        assert!(span.style.add_modifier == Modifier::BOLD);
    }

    #[test]
    fn styled_text_renders_in_terminal() {
        use ratatui::backend::TestBackend;
        use ratatui::widgets::{Paragraph, Widget};
        let backend = TestBackend::new(10, 1);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                let span = "hello".blue();
                let paragraph = Paragraph::new(ratatui::text::Line::from(span));
                paragraph.render(f.area(), f.buffer_mut());
            })
            .unwrap();
        let cell = &terminal.backend().buffer()[(0, 0)];
        assert_eq!(cell.symbol(), "h");
        assert_eq!(cell.fg, Color::Blue);
    }
}
