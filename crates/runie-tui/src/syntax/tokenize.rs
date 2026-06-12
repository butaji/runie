//! Tokenizer for syntax highlighting

use crate::syntax::keywords::{
    Language, BASH_KEYWORDS, C_KEYWORDS, C_TYPES, GO_FUNCTIONS, GO_KEYWORDS, GO_TYPES,
    JAVA_FUNCTIONS, JAVA_KEYWORDS, JAVA_TYPES, JS_FUNCTIONS, JS_KEYWORDS, JS_TYPES,
    PYTHON_BUILTINS, PYTHON_FUNCTIONS, PYTHON_KEYWORDS, RUST_KEYWORDS, RUST_TYPES, SQL_KEYWORDS,
};
use ratatui::style::{Color, Modifier, Style};

/// A highlighted token with its style.
#[derive(Debug, Clone, PartialEq)]
pub struct SyntaxToken {
    pub content: String,
    pub style: Style,
}

/// Syntax highlighting styles.
mod highlight_styles {
    use super::*;

    pub fn keyword() -> Style {
        Style::default()
            .fg(Color::Indexed(147))
            .add_modifier(Modifier::BOLD)
    }

    pub fn string() -> Style {
        Style::default().fg(Color::Indexed(114))
    }

    pub fn number() -> Style {
        Style::default().fg(Color::Indexed(175))
    }

    pub fn comment() -> Style {
        Style::default()
            .fg(Color::Indexed(245))
            .add_modifier(Modifier::ITALIC)
    }

    pub fn type_() -> Style {
        Style::default().fg(Color::Indexed(75))
    }

    pub fn function() -> Style {
        Style::default().fg(Color::Indexed(111))
    }

    pub fn plain() -> Style {
        Style::default()
    }
}

fn is_identifier_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Tokenize a line of code into syntax tokens.
pub fn tokenize_line(line: &str, lang: Language) -> Vec<SyntaxToken> {
    let mut tokens = Vec::new();
    let mut chars = line.chars().peekable();
    let mut current = String::new();
    let mut in_string = None; // Some(char) for quote char
    let in_comment = false;
    let mut in_block_comment = false;

    macro_rules! flush_and_add {
        ($style:expr) => {
            if !current.is_empty() {
                tokens.push(SyntaxToken {
                    content: std::mem::take(&mut current),
                    style: $style,
                });
            }
        };
    }

    while let Some(c) = chars.next() {
        // Handle string literals
        if let Some(quote) = in_string {
            current.push(c);
            if c == quote && current.len() > 1 && !current[..current.len() - 1].ends_with('\\') {
                flush_and_add!(highlight_styles::string());
                in_string = None;
            }
            continue;
        }

        // Handle line comments
        if in_comment {
            current.push(c);
            continue;
        }

        // Handle block comments
        if in_block_comment {
            current.push(c);
            if c == '*' && chars.peek() == Some(&'/') {
                current.push(chars.next().unwrap());
                flush_and_add!(highlight_styles::comment());
                in_block_comment = false;
            }
            continue;
        }

        // Check for comment start
        if c == '/' {
            if chars.peek() == Some(&'/') {
                flush_and_add!(highlight_styles::plain());
                current.push(c);
                current.push(chars.next().unwrap());
                flush_and_add!(highlight_styles::comment());
                for cc in chars.by_ref() {
                    current.push(cc);
                }
                flush_and_add!(highlight_styles::comment());
                break;
            }
            if chars.peek() == Some(&'*') {
                flush_and_add!(highlight_styles::plain());
                current.push(c);
                current.push(chars.next().unwrap());
                flush_and_add!(highlight_styles::comment());
                in_block_comment = true;
                continue;
            }
        }

        // Check for string start
        if c == '"' || c == '\'' {
            flush_and_add!(highlight_styles::plain());
            current.push(c);
            in_string = Some(c);
            continue;
        }

        // Check for numbers
        if c.is_ascii_digit() {
            flush_and_add!(highlight_styles::plain());
            current.push(c);
            while let Some(&cc) = chars.peek() {
                if cc.is_ascii_alphanumeric() || cc == '.' || cc == '_' {
                    current.push(chars.next().unwrap());
                } else {
                    break;
                }
            }
            flush_and_add!(highlight_styles::number());
            continue;
        }

        // Check for identifiers and keywords
        if c.is_alphabetic() || c == '_' {
            flush_and_add!(highlight_styles::plain());
            current.push(c);
            while let Some(&cc) = chars.peek() {
                if is_identifier_char(cc) {
                    current.push(chars.next().unwrap());
                } else {
                    break;
                }
            }
            let word = current.clone();
            let style = classify_word(&word, lang);
            flush_and_add!(style);
            continue;
        }

        current.push(c);
    }

    flush_and_add!(highlight_styles::plain());
    tokens
}

/// Classify a word as keyword, type, function, or plain identifier.
fn classify_word(word: &str, lang: Language) -> Style {
    use highlight_styles::*;

    if is_keyword(word, lang) {
        return keyword();
    }
    if is_type(word, lang) {
        return type_();
    }
    if is_function(word, lang) {
        return function();
    }

    // Check if it looks like a type (PascalCase or starts with uppercase)
    if word
        .chars()
        .next()
        .map(|c| c.is_uppercase())
        .unwrap_or(false)
        && word
            .chars()
            .nth(1)
            .map(|c| c.is_lowercase())
            .unwrap_or(false)
    {
        return type_();
    }

    plain()
}

/// Check if a word is a keyword in the given language.
fn is_keyword(word: &str, lang: Language) -> bool {
    match lang {
        Language::Rust => RUST_KEYWORDS.contains(&word),
        Language::Python => PYTHON_KEYWORDS.contains(&word),
        Language::JavaScript | Language::TypeScript => JS_KEYWORDS.contains(&word),
        Language::Go => GO_KEYWORDS.contains(&word),
        Language::Java => JAVA_KEYWORDS.contains(&word),
        Language::C | Language::Cpp => C_KEYWORDS.contains(&word),
        Language::Sql => SQL_KEYWORDS.contains(&word),
        Language::Bash => BASH_KEYWORDS.contains(&word),
        Language::Json
        | Language::Yaml
        | Language::Toml
        | Language::Html
        | Language::Xml
        | Language::Css
        | Language::Markdown
        | Language::Plain => false,
    }
}

/// Check if a word is a type name in the given language.
fn is_type(word: &str, lang: Language) -> bool {
    match lang {
        Language::Rust => RUST_TYPES.contains(&word),
        Language::Python => PYTHON_BUILTINS.contains(&word),
        Language::JavaScript | Language::TypeScript => JS_TYPES.contains(&word),
        Language::Go => GO_TYPES.contains(&word),
        Language::Java => JAVA_TYPES.contains(&word),
        Language::C | Language::Cpp => C_TYPES.contains(&word),
        Language::Json
        | Language::Yaml
        | Language::Toml
        | Language::Html
        | Language::Xml
        | Language::Css
        | Language::Markdown
        | Language::Bash
        | Language::Sql
        | Language::Plain => false,
    }
}

/// Check if a word is a built-in function in the given language.
fn is_function(word: &str, lang: Language) -> bool {
    match lang {
        Language::Rust => false,
        Language::Python => PYTHON_FUNCTIONS.contains(&word),
        Language::JavaScript | Language::TypeScript => JS_FUNCTIONS.contains(&word),
        Language::Go => GO_FUNCTIONS.contains(&word),
        Language::Java => JAVA_FUNCTIONS.contains(&word),
        Language::C | Language::Cpp => false,
        Language::Json
        | Language::Yaml
        | Language::Toml
        | Language::Html
        | Language::Xml
        | Language::Css
        | Language::Markdown
        | Language::Bash
        | Language::Sql
        | Language::Plain => false,
    }
}

/// Highlight code content with syntax tokens.
pub fn highlight_code(content: &str, lang: &str) -> Vec<Vec<SyntaxToken>> {
    let language = Language::from_fence(lang);
    content
        .lines()
        .map(|line| tokenize_line(line, language))
        .collect()
}
