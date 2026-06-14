//! Tests for syntect-based syntax highlighting.

#[cfg(test)]
mod tests {
    use crate::syntax::highlight_code;
    use ratatui::style::Color;

    #[test]
    fn highlight_code_multiline() {
        let code = "fn main() {\n    let x = 42;\n}";
        let lines = highlight_code(code, "rust");
        assert_eq!(lines.len(), 3, "Should have 3 lines");
        assert!(!lines[0].is_empty(), "First line should have tokens");
    }

    #[test]
    fn highlight_rust_shows_keyword_colors() {
        let code = "pub fn example() { let x = 1; }";
        let lines = highlight_code(code, "rust");
        let tokens: Vec<_> = lines.into_iter().flatten().collect();
        let keyword_colors: Vec<_> = tokens
            .iter()
            .filter(|t| ["fn", "let", "pub"].contains(&t.content.as_str()))
            .map(|t| t.style.fg)
            .collect();
        assert!(
            keyword_colors
                .iter()
                .all(|c| c.is_some() && *c != Some(Color::Reset)),
            "Keywords should have colors: {:?}",
            keyword_colors
        );
    }

    #[test]
    fn highlight_python_shows_def() {
        let code = "def hello():\n    pass";
        let lines = highlight_code(code, "python");
        let tokens: Vec<_> = lines.into_iter().flatten().collect();
        let def_token = tokens.iter().find(|t| t.content == "def");
        assert!(def_token.is_some(), "Should have 'def' token");
        assert!(
            def_token.unwrap().style.fg.is_some(),
            "'def' should have a foreground color"
        );
    }

    #[test]
    fn highlight_unknown_language_does_not_panic() {
        let lines = highlight_code("some text", "not-a-real-language");
        assert_eq!(lines.len(), 1);
        assert!(!lines[0].is_empty());
    }
}
