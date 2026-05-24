use syntect::{
    highlighting::{ThemeSet, SyntaxSet},
    easy::HighlightLines,
    util::LinesWithEndings,
};
use ratatui::{
    style::{Style, Color},
    text::{Line, Span},
};

lazy_static::lazy_static! {
    static ref SYNTAX_SET: SyntaxSet = SyntaxSet::load_defaults_newlines();
    static ref THEME_SET: ThemeSet = ThemeSet::load_defaults();
}

pub fn highlight_code(code: &str, language: &str) -> Vec<Line<'static>> {
    let syntax = SYNTAX_SET.find_syntax_by_token(language)
        .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());

    let theme = &THEME_SET.themes["base16-ocean.dark"];
    let mut highlighter = HighlightLines::new(syntax, theme);

    let mut lines = Vec::new();
    for line in LinesWithEndings::from(code) {
        let highlighted = highlighter.highlight_line(line, &SYNTAX_SET).unwrap();
        let spans: Vec<Span> = highlighted.into_iter().map(|(style, text)| {
            let color = Color::Rgb(
                style.foreground.r,
                style.foreground.g,
                style.foreground.b,
            );
            Span::styled(text.to_string(), Style::default().fg(color))
        }).collect();
        lines.push(Line::from(spans));
    }

    lines
}
