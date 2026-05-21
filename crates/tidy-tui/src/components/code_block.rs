use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Widget,
};
use crate::theme::ThemeWrapper;

#[derive(Clone)]
pub struct CodeBlock {
    pub lines: Vec<CodeLine>,
    pub start_line: usize,
    pub language: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CodeLine {
    pub number: usize,
    pub text: String,
    pub status: LineStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LineStatus {
    Normal,
    Added,
    Removed,
    Highlighted,
}

impl Default for CodeBlock {
    fn default() -> Self {
        Self {
            lines: Vec::new(),
            start_line: 1,
            language: None,
        }
    }
}

struct StyleHelpers {
    text_body: Style,
    text_tertiary: Style,
    bg_item: Style,
    syntax_string: Style,
    syntax_attr: Style,
    bg_panel: Style,
}

impl StyleHelpers {
    fn new(theme: &ThemeWrapper) -> Self {
        Self {
            text_body: Style::default().fg(theme.color("text.secondary").into()),
            text_tertiary: Style::default().fg(theme.color("text.dim").into()),
            bg_item: Style::default().bg(theme.color("bg.selection").into()),
            syntax_string: Style::default().fg(theme.color("success").into()),
            syntax_attr: Style::default().fg(theme.color("code.path").into()),
            bg_panel: Style::default().bg(theme.color("bg.panel").into()),
        }
    }
    fn primary(&self) -> Style {
        self.text_body
    }
    fn code_line_number(&self) -> Style {
        self.text_tertiary
    }
    fn code_added(&self) -> Style {
        Style::default().bg(Color::Rgb(26, 60, 26))
    }
    fn code_removed(&self) -> Style {
        Style::default().bg(Color::Rgb(60, 26, 26))
    }
    fn bg_highlight(&self) -> Style {
        self.bg_item
    }
    fn syntax_string(&self) -> Style {
        self.syntax_string
    }
    fn syntax_attr(&self) -> Style {
        self.syntax_attr
    }
    fn bg_code(&self) -> Style {
        self.bg_panel
    }
}

impl Widget for CodeBlock {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = ThemeWrapper::default();
        let sp = StyleHelpers::new(&theme);
        let line_num_width = 4;

        for (i, line) in self.lines.iter().enumerate() {
            let y = area.y + i as u16;
            if y >= area.y + area.height {
                break;
            }

            let bg_style = match line.status {
                LineStatus::Added => sp.code_added(),
                LineStatus::Removed => sp.code_removed(),
                LineStatus::Highlighted => sp.bg_highlight(),
                LineStatus::Normal => sp.bg_code(),
            };

            for x in area.x..area.x + area.width {
                buf.get_mut(x, y).set_style(bg_style);
            }

            let num_str = format!("{:>3} ", line.number);
            let num_line = Line::from(vec![Span::styled(&num_str, sp.code_line_number())]);
            buf.set_line(area.x, y, &num_line, line_num_width);

            let highlighted = self.highlight_line(&line.text, &theme);
            let content_line = Line::from(highlighted);
            buf.set_line(area.x + line_num_width, y, &content_line, area.width - line_num_width);
        }
    }
}

impl CodeBlock {
    fn highlight_line(&self, text: &str, theme: &ThemeWrapper) -> Vec<Span<'static>> {
        let sp = StyleHelpers::new(theme);
        let mut parts = Vec::new();

        let mut current = String::new();
        let mut in_string = false;

        for ch in text.chars() {
            if ch == '"' || ch == '\'' {
                if !current.is_empty() {
                    parts.push(Span::styled(current.clone(), sp.primary()));
                    current.clear();
                }
                in_string = !in_string;
                current.push(ch);
                if !in_string {
                    parts.push(Span::styled(current.clone(), sp.syntax_string()));
                    current.clear();
                }
            } else if in_string {
                current.push(ch);
            } else if ch.is_numeric() && (current.is_empty() || current.chars().all(|c| c.is_numeric())) {
                current.push(ch);
            } else {
                if !current.is_empty() && current.chars().all(|c| c.is_numeric()) {
                    parts.push(Span::styled(current.clone(), sp.syntax_attr()));
                    current.clear();
                }
                current.push(ch);
            }
        }

        if !current.is_empty() {
            if current.chars().all(|c| c.is_numeric()) {
                parts.push(Span::styled(current.clone(), sp.syntax_attr()));
            } else {
                parts.push(Span::styled(current.clone(), sp.primary()));
            }
        }

        if parts.is_empty() {
            parts.push(Span::styled(text.to_string(), sp.primary()));
        }

        parts
    }
}
