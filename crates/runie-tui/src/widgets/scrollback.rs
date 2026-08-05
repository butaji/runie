//! Scrollback widget: append-only transcript with autoscroll.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line as RatLine;
use ratatui::widgets::{Paragraph, Widget, Wrap};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    User,
    Assistant,
    Tool,
    ToolResult,
    System,
}

impl LineKind {
    pub fn style(self) -> Style {
        match self {
            LineKind::User => Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            LineKind::Assistant => Style::default().fg(Color::Green),
            LineKind::Tool => Style::default().fg(Color::Yellow),
            LineKind::ToolResult => Style::default().fg(Color::Magenta),
            LineKind::System => Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
        }
    }

    pub fn prefix(self) -> &'static str {
        match self {
            LineKind::User => "user> ",
            LineKind::Assistant => "assistant> ",
            LineKind::Tool => "  ⚙ ",
            LineKind::ToolResult => "  ↳ ",
            LineKind::System => "* ",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Line {
    pub kind: LineKind,
    pub text: String,
}

impl Line {
    pub fn new(kind: LineKind, text: impl Into<String>) -> Self {
        Self { kind, text: text.into() }
    }
}

#[derive(Debug, Default)]
pub struct Scrollback {
    lines: Vec<Line>,
    autoscroll: bool,
    scroll_offset: usize,
}

impl Scrollback {
    pub fn new() -> Self {
        Self { lines: Vec::new(), autoscroll: true, scroll_offset: 0 }
    }

    pub fn append(&mut self, line: Line) {
        self.lines.push(line);
        if self.autoscroll {
            // Hold offset so the tail is in view after the next render
            // (the actual clamp happens in `render` once we know area height).
            self.scroll_offset = self.lines.len();
        }
    }

    pub fn clear(&mut self) {
        self.lines.clear();
        self.scroll_offset = 0;
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Borrow the lines (for tests).
    pub fn lines(&self) -> &[Line] {
        &self.lines
    }

    /// Find the index of the first line whose `text` contains the needle.
    pub fn find_first_containing(&self, needle: &str) -> Option<usize> {
        self.lines.iter().position(|l| l.text.contains(needle))
    }

    /// Find all line indices whose `text` contains the needle.
    pub fn find_all_containing(&self, needle: &str) -> Vec<usize> {
        self.lines
            .iter()
            .enumerate()
            .filter_map(|(i, l)| if l.text.contains(needle) { Some(i) } else { None })
            .collect()
    }

    /// Mutable reference to the last line of `kind`, if any.
    pub fn last_mut_by_kind(&mut self, kind: LineKind) -> Option<&mut Line> {
        self.lines.iter_mut().rev().find(|l| l.kind == kind)
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        // Wrap-aware: each Line is one logical row that may wrap to multiple
        // physical rows. We approximate by giving each line 1 "slot" plus
        // overflow based on text length and area width.
        let width = area.width.saturating_sub(2) as usize; // account for prefix
        let mut physical_rows: Vec<(LineKind, String)> = Vec::new();
        for line in &self.lines {
            let prefixed = format!("{}{}", line.kind.prefix(), line.text);
            if width == 0 || prefixed.chars().count() <= width {
                physical_rows.push((line.kind, prefixed));
            } else {
                // Wrap long lines by character count.
                let mut chars: Vec<char> = prefixed.chars().collect();
                while !chars.is_empty() {
                    let take = width.min(chars.len());
                    let head: String = chars.drain(..take).collect();
                    physical_rows.push((line.kind, head));
                }
            }
        }

        let total = physical_rows.len();
        let visible = area.height as usize;
        // Clamp scroll_offset so the tail is visible.
        if total > visible {
            let max_offset = total - visible;
            if self.scroll_offset > max_offset {
                self.scroll_offset = max_offset;
            }
            if self.scroll_offset == 0 && self.autoscroll {
                self.scroll_offset = max_offset;
            }
        } else {
            self.scroll_offset = 0;
        }

        let start = self.scroll_offset;
        let end = (start + visible).min(total);

        if start >= end {
            // Nothing to render. Avoid passing an empty slice to ratatui's
            // Paragraph/Line, which can panic on some versions.
            return;
        }

        for (row, (kind, text)) in physical_rows[start..end].iter().enumerate() {
            let line = RatLine::from(text.as_str()).style(kind.style());
            Paragraph::new(line)
                .wrap(Wrap { trim: false })
                .render(Rect { x: area.x, y: area.y + row as u16, width: area.width, height: 1 }, buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_and_len() {
        let mut s = Scrollback::new();
        s.append(Line::new(LineKind::User, "hi"));
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn clear_empties() {
        let mut s = Scrollback::new();
        s.append(Line::new(LineKind::User, "hi"));
        s.clear();
        assert!(s.is_empty());
    }

    #[test]
    fn find_first_containing() {
        let mut s = Scrollback::new();
        s.append(Line::new(LineKind::Assistant, "Hello world"));
        s.append(Line::new(LineKind::Assistant, "Goodbye world"));
        assert_eq!(s.find_first_containing("world"), Some(0));
    }
}