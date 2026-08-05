//! 1-row status bar.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Widget};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    Ready,
    Thinking,
    Streaming,
    Aborted,
    Error(String),
}

impl Status {
    pub fn label(&self) -> String {
        match self {
            Self::Ready => "ready".into(),
            Self::Thinking => "thinking...".into(),
            Self::Streaming => "streaming".into(),
            Self::Aborted => "aborted".into(),
            Self::Error(e) => format!("error: {e}"),
        }
    }

    pub fn style(&self) -> Style {
        match self {
            Self::Ready => Style::default().fg(Color::Green),
            Self::Thinking => Style::default().fg(Color::Yellow),
            Self::Streaming => Style::default().fg(Color::Blue),
            Self::Aborted => Style::default().fg(Color::DarkGray),
            Self::Error(_) => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        }
    }
}

impl Default for Status {
    fn default() -> Self {
        Self::Ready
    }
}

#[derive(Debug, Default)]
pub struct StatusBar {
    state: Status,
}

impl StatusBar {
    pub fn new() -> Self {
        Self { state: Status::default() }
    }

    pub fn set(&mut self, s: Status) {
        self.state = s;
    }

    pub fn current(&self) -> &Status {
        &self.state
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        // grok-build's minimal-mode status row: dim left-side state + bright
        // right-side badge. We render two spans: state on the left, "runie v0.1.0"
        // on the right (acts as our model pill).
        use ratatui::text::Span;
        let width = area.width as usize;
        let left = format!("{} · ctrl+o transcript", self.state.label());
        let right = "runie v0.1.0";
        let pad = width.saturating_sub(left.len() + right.len());
        let line = Line::from(vec![
            Span::styled(left, self.state.style()),
            Span::raw(" ".repeat(pad.max(1))),
            Span::styled(right, Style::default().fg(Color::Cyan)),
        ]);
        let p = Paragraph::new(line);
        Widget::render(p, area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_ready() {
        assert_eq!(StatusBar::new().current(), &Status::Ready);
    }

    #[test]
    fn label_distinct_per_variant() {
        let variants = [
            Status::Ready,
            Status::Thinking,
            Status::Streaming,
            Status::Aborted,
            Status::Error("x".into()),
        ];
        let labels: Vec<_> = variants.iter().map(Status::label).collect();
        let unique: std::collections::HashSet<_> = labels.iter().collect();
        assert_eq!(unique.len(), labels.len());
    }
}