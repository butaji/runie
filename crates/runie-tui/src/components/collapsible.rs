use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Widget,
};
use crate::theme::ThemeWrapper;

#[derive(Clone)]
pub struct Collapsible {
    pub title: String,
    pub expanded: bool,
    pub content_lines: Vec<String>,
    pub theme: ThemeWrapper,
}

impl Default for Collapsible {
    fn default() -> Self {
        Self {
            title: String::new(),
            expanded: false,
            content_lines: Vec::new(),
            theme: ThemeWrapper::default(),
        }
    }
}

struct StyleHelpers {
    text_body: Style,
    text_tertiary: Style,
}

impl StyleHelpers {
    fn new(theme: &ThemeWrapper) -> Self {
        Self {
            text_body: Style::default().fg(theme.color("text.secondary").into()),
            text_tertiary: Style::default().fg(theme.color("text.dim").into()),
        }
    }
    fn primary(&self) -> Style {
        self.text_body
    }
    fn tertiary(&self) -> Style {
        self.text_tertiary
    }
}

impl Widget for Collapsible {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let sp = StyleHelpers::new(&self.theme);
        let mut y = area.y;

        let symbol = if self.expanded { "\u{25BC}" } else { "\u{25B6}" };
        let header_parts = vec![
            Span::styled(symbol, sp.tertiary()),
            Span::raw(" "),
            Span::styled(&self.title, sp.primary()),
        ];
        let header_line = Line::from(header_parts);
        buf.set_line(area.x, y, &header_line, area.width);
        y += 1;

        if self.expanded {
            for line in &self.content_lines {
                if y >= area.y + area.height {
                    break;
                }
                let content_line = Line::from(vec![Span::styled(line.as_str(), sp.primary())]);
                buf.set_line(area.x + 2, y, &content_line, area.width - 2);
                y += 1;
            }
        }
    }
}

impl Collapsible {
    pub fn toggle(&mut self) {
        self.expanded = !self.expanded;
    }
}
