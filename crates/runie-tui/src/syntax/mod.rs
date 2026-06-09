//! Syntax highlighting for code blocks
//!
//! Provides tokenization and keyword highlighting for common programming languages.

mod keywords;
mod tokenize;

pub use keywords::Language;
pub use tokenize::{highlight_code, SyntaxToken};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_keyword_highlighting() {
        let line = "fn main() {}";
        let tokens = tokenize::tokenize_line(line, Language::Rust);
        let contents: Vec<_> = tokens.iter().map(|t| t.content.as_str()).collect();
        assert!(contents.contains(&"fn"), "Should have fn keyword");
        assert!(contents.contains(&"main"), "Should have function name");
        let full: String = tokens.iter().map(|t| t.content.as_str()).collect();
        assert!(full.contains('{'), "Should have brace");
    }

    #[test]
    fn python_keyword_highlighting() {
        let line = "def hello():";
        let tokens = tokenize::tokenize_line(line, Language::Python);
        let contents: Vec<_> = tokens.iter().map(|t| t.content.as_str()).collect();
        assert!(contents.contains(&"def"), "Should have def keyword");
        assert!(contents.contains(&"hello"), "Should have function name");
    }

    #[test]
    fn js_string_highlighting() {
        use ratatui::style::Color;
        let line = r#"let msg = "hello";"#;
        let tokens = tokenize::tokenize_line(line, Language::JavaScript);
        let string_token = tokens.iter().find(|t| t.content == "\"hello\"");
        assert!(string_token.is_some(), "Should have string token");
        assert_eq!(
            string_token.unwrap().style.fg,
            Some(Color::Indexed(114)),
            "String should be green"
        );
    }

    #[test]
    fn number_highlighting() {
        
        let line = "let x = 42;";
        let tokens = tokenize::tokenize_line(line, Language::JavaScript);
        let num_token = tokens.iter().find(|t| t.content == "42");
        assert!(num_token.is_some(), "Should have number token");
    }

    #[test]
    fn comment_highlighting() {
        
        let line = "// this is a comment";
        let tokens = tokenize::tokenize_line(line, Language::Rust);
        let comment_token = tokens.iter().find(|t| t.content.contains("this is a comment"));
        assert!(comment_token.is_some(), "Should have comment token");
    }

    #[test]
    fn language_detection() {
        assert_eq!(Language::from_fence("rust"), Language::Rust);
        assert_eq!(Language::from_fence("rs"), Language::Rust);
        assert_eq!(Language::from_fence("python"), Language::Python);
        assert_eq!(Language::from_fence("py"), Language::Python);
        assert_eq!(Language::from_fence("javascript"), Language::JavaScript);
        assert_eq!(Language::from_fence("js"), Language::JavaScript);
        assert_eq!(Language::from_fence("typescript"), Language::TypeScript);
        assert_eq!(Language::from_fence("ts"), Language::TypeScript);
        assert_eq!(Language::from_fence("go"), Language::Go);
        assert_eq!(Language::from_fence("java"), Language::Java);
        assert_eq!(Language::from_fence("c"), Language::C);
        assert_eq!(Language::from_fence("cpp"), Language::Cpp);
        assert_eq!(Language::from_fence("sql"), Language::Sql);
        assert_eq!(Language::from_fence("bash"), Language::Bash);
        assert_eq!(Language::from_fence("sh"), Language::Bash);
        assert_eq!(Language::from_fence("unknown"), Language::Plain);
    }

    #[test]
    fn multi_language_highlight() {
        let rust_code = "let x: i32 = 42;";
        let tokens = tokenize::tokenize_line(rust_code, Language::Rust);
        assert!(!tokens.is_empty(), "Should have tokens");

        let python_code = "x = 42";
        let py_tokens = tokenize::tokenize_line(python_code, Language::Python);
        assert!(!py_tokens.is_empty(), "Should have python tokens");
    }

    #[test]
    fn highlight_code_multiline() {
        let code = "fn main() {\n    let x = 42;\n}";
        let lines = highlight_code(code, "rust");
        assert_eq!(lines.len(), 3, "Should have 3 lines");
        assert!(!lines[0].is_empty(), "First line should have tokens");
    }

    #[test]
    fn sql_keyword_highlighting() {
        let line = "SELECT * FROM users";
        let tokens = tokenize::tokenize_line(line, Language::Sql);
        let contents: Vec<_> = tokens.iter().map(|t| t.content.as_str()).collect();
        assert!(contents.contains(&"SELECT"), "Should have SELECT keyword");
        assert!(contents.contains(&"FROM"), "Should have FROM keyword");
    }

    #[test]
    fn go_keyword_highlighting() {
        let line = "package main";
        let tokens = tokenize::tokenize_line(line, Language::Go);
        let contents: Vec<_> = tokens.iter().map(|t| t.content.as_str()).collect();
        assert!(contents.contains(&"package"), "Should have package keyword");
        assert!(contents.contains(&"main"), "Should have main identifier");
    }

    #[test]
    fn type_highlighting() {
        let line = "let name: String = String::new();";
        let tokens = tokenize::tokenize_line(line, Language::Rust);
        let type_token = tokens.iter().find(|t| t.content == "String");
        assert!(type_token.is_some(), "Should have String type token");
    }

    #[test]
    fn empty_line() {
        let tokens = tokenize::tokenize_line("", Language::Rust);
        assert!(tokens.is_empty() || tokens.len() == 1, "Empty line should have minimal tokens");
    }
}
