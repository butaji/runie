//! Syntax highlighting for code blocks
//!
//! Uses `syntect` for real syntax highlighting.

use std::sync::OnceLock;

use ratatui::style::{Color, Modifier, Style};
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, Theme, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

/// A highlighted token with its style.
#[derive(Debug, Clone, PartialEq)]
pub struct SyntaxToken {
    pub content: String,
    pub style: Style,
}

fn syntax_set() -> &'static SyntaxSet {
    static SET: OnceLock<SyntaxSet> = OnceLock::new();
    SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn theme_set() -> &'static ThemeSet {
    static SET: OnceLock<ThemeSet> = OnceLock::new();
    SET.get_or_init(ThemeSet::load_defaults)
}

fn theme() -> &'static Theme {
    static THEME: OnceLock<Theme> = OnceLock::new();
    THEME.get_or_init(|| {
        let ts = theme_set();
        let name = ["base16-ocean.dark", "base16-mocha.dark"]
            .into_iter()
            .find(|n| ts.themes.contains_key(*n))
            .or_else(|| ts.themes.keys().next().map(String::as_str))
            .expect("theme set contains at least one theme");
        ts.themes
            .get(name)
            .expect("theme name came from map")
            .clone()
    })
}

fn syntect_color_to_ratatui(c: syntect::highlighting::Color) -> Color {
    Color::Rgb(c.r, c.g, c.b)
}

fn convert_style(s: syntect::highlighting::Style) -> Style {
    let mut style = Style::default()
        .fg(syntect_color_to_ratatui(s.foreground))
        .bg(syntect_color_to_ratatui(s.background));
    if s.font_style.contains(FontStyle::BOLD) {
        style = style.add_modifier(Modifier::BOLD);
    }
    if s.font_style.contains(FontStyle::ITALIC) {
        style = style.add_modifier(Modifier::ITALIC);
    }
    if s.font_style.contains(FontStyle::UNDERLINE) {
        style = style.add_modifier(Modifier::UNDERLINED);
    }
    style
}

/// Highlight code content with syntax tokens.
pub fn highlight_code(code: &str, lang: &str) -> Vec<Vec<SyntaxToken>> {
    let ss = syntax_set();
    let syntax = ss
        .find_syntax_by_token(lang)
        .or_else(|| ss.find_syntax_by_extension(lang))
        .unwrap_or_else(|| ss.find_syntax_plain_text());
    let mut highlighter = HighlightLines::new(syntax, theme());
    LinesWithEndings::from(code)
        .map(|line| {
            highlighter
                .highlight_line(line, ss)
                .unwrap_or_default()
                .into_iter()
                .map(|(style, content)| SyntaxToken {
                    content: content.to_string(),
                    style: convert_style(style),
                })
                .collect()
        })
        .collect()
}
