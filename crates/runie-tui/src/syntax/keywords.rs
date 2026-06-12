//! Keyword definitions for syntax highlighting

use std::fmt;
use std::hash::Hash;
use std::str;

// Keyword modules

macro_rules! lang {
    ($mod:ident, $($item:ident),+ $(,)?) => {
        pub mod $mod;
        $(pub use $mod::$item;)+
    };
}

lang!(bash, BASH_KEYWORDS);
lang!(c, C_KEYWORDS, C_TYPES);
lang!(go, GO_FUNCTIONS, GO_KEYWORDS, GO_TYPES);
lang!(java, JAVA_FUNCTIONS, JAVA_KEYWORDS, JAVA_TYPES);
lang!(js, JS_FUNCTIONS, JS_KEYWORDS, JS_TYPES);
lang!(python, PYTHON_BUILTINS, PYTHON_FUNCTIONS, PYTHON_KEYWORDS);
lang!(rust, RUST_KEYWORDS, RUST_TYPES);
lang!(sql, SQL_KEYWORDS);

/// Language identifiers supported for syntax highlighting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Java,
    C,
    Cpp,
    Markdown,
    Json,
    Yaml,
    Bash,
    Sql,
    Html,
    Css,
    Xml,
    Toml,
    Plain,
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Language::Rust => write!(f, "rust"),
            Language::Python => write!(f, "python"),
            Language::JavaScript => write!(f, "javascript"),
            Language::TypeScript => write!(f, "typescript"),
            Language::Go => write!(f, "go"),
            Language::Java => write!(f, "java"),
            Language::C => write!(f, "c"),
            Language::Cpp => write!(f, "cpp"),
            Language::Markdown => write!(f, "markdown"),
            Language::Json => write!(f, "json"),
            Language::Yaml => write!(f, "yaml"),
            Language::Bash => write!(f, "bash"),
            Language::Sql => write!(f, "sql"),
            Language::Html => write!(f, "html"),
            Language::Css => write!(f, "css"),
            Language::Xml => write!(f, "xml"),
            Language::Toml => write!(f, "toml"),
            Language::Plain => write!(f, "plain"),
        }
    }
}

impl Language {
    /// Detect language from a fence label string.
    pub fn from_fence(fence: &str) -> Self {
        match fence.to_lowercase().as_str() {
            "rs" | "rust" => Language::Rust,
            "py" | "python" => Language::Python,
            "js" | "javascript" => Language::JavaScript,
            "ts" | "typescript" => Language::TypeScript,
            "go" | "golang" => Language::Go,
            "java" => Language::Java,
            "c" => Language::C,
            "cpp" | "c++" | "cc" => Language::Cpp,
            "md" | "markdown" => Language::Markdown,
            "json" => Language::Json,
            "yaml" | "yml" => Language::Yaml,
            "sh" | "bash" | "shell" | "zsh" => Language::Bash,
            "sql" => Language::Sql,
            "html" | "htm" => Language::Html,
            "css" => Language::Css,
            "xml" => Language::Xml,
            "toml" => Language::Toml,
            _ => Language::Plain,
        }
    }
}

