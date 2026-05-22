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
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_style(bg_style);
                }
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
        let tokens = Self::tokenize(text);
        Self::build_spans(tokens, &sp)
    }

    fn tokenize(text: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut current = String::new();
        let mut in_string = false;

        for ch in text.chars() {
            Self::process_char(ch, &mut current, &mut in_string, &mut tokens);
        }

        if !current.is_empty() {
            if current.chars().all(|c| c.is_numeric()) {
                tokens.push(Token::Number(current));
            } else {
                tokens.push(Token::Text(current));
            }
        }

        tokens
    }

    fn process_char(ch: char, current: &mut String, in_string: &mut bool, tokens: &mut Vec<Token>) {
        let is_quote = ch == '"' || ch == '\'';

        if is_quote {
            Self::handle_quote(current, in_string, tokens);
        } else if *in_string {
            current.push(ch);
        } else if ch.is_numeric() && current.chars().all(|c| c.is_numeric()) {
            current.push(ch);
        } else {
            Self::flush_numeric_and_push(current, tokens, ch);
        }
    }

    fn handle_quote(current: &mut String, in_string: &mut bool, tokens: &mut Vec<Token>) {
        if !current.is_empty() {
            tokens.push(Token::Text(std::mem::take(current)));
        }
        *in_string = !*in_string;
        current.push('\'');
        if !*in_string {
            tokens.push(Token::String(std::mem::take(current)));
        }
    }

    fn flush_numeric_and_push(current: &mut String, tokens: &mut Vec<Token>, ch: char) {
        if !current.is_empty() && current.chars().all(|c| c.is_numeric()) {
            tokens.push(Token::Number(std::mem::take(current)));
        }
        current.push(ch);
    }

    fn build_spans(tokens: Vec<Token>, sp: &StyleHelpers) -> Vec<Span<'static>> {
        let mut parts = Vec::new();

        for token in tokens {
            match token {
                Token::Text(s) => parts.push(Span::styled(s, sp.primary())),
                Token::String(s) => parts.push(Span::styled(s, sp.syntax_string())),
                Token::Number(s) => parts.push(Span::styled(s, sp.syntax_attr())),
            }
        }

        if parts.is_empty() {
            parts.push(Span::styled(String::new(), sp.primary()));
        }

        parts
    }
}

enum Token {
    Text(String),
    String(String),
    Number(String),
}
