//! Syntax highlighting for code blocks
//!
//! Provides tokenization and keyword highlighting for common programming languages.

use ratatui::style::{Color, Modifier, Style};

/// A highlighted token with its style.
#[derive(Debug, Clone, PartialEq)]
pub struct SyntaxToken {
    pub content: String,
    pub style: Style,
}

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

/// Syntax highlighting styles.
mod highlight_styles {
    use super::*;

    pub fn keyword() -> Style {
        Style::default().fg(Color::Indexed(147)).add_modifier(Modifier::BOLD) // Light magenta
    }

    pub fn string() -> Style {
        Style::default().fg(Color::Indexed(114)) // Green
    }

    pub fn number() -> Style {
        Style::default().fg(Color::Indexed(175)) // Light green
    }

    pub fn comment() -> Style {
        Style::default().fg(Color::Indexed(245)).add_modifier(Modifier::ITALIC) // Gray italic
    }

    pub fn type_() -> Style {
        Style::default().fg(Color::Indexed(75)) // Cyan
    }

    pub fn function() -> Style {
        Style::default().fg(Color::Indexed(111)) // Light cyan
    }

    pub fn operator() -> Style {
        Style::default().fg(Color::Indexed(139)) // Gray-violet
    }

    pub fn attribute() -> Style {
        Style::default().fg(Color::Indexed(181)) // Light orange
    }

    pub fn plain() -> Style {
        Style::default()
    }
}

fn is_identifier_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Tokenize a line of code into syntax tokens.
fn tokenize_line(line: &str, lang: Language) -> Vec<SyntaxToken> {
    let mut tokens = Vec::new();
    let mut chars = line.chars().peekable();
    let mut current = String::new();
    let mut in_string = None; // Some(char) for quote char
    let mut in_comment = false;
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
            if c == quote && current.len() > 1 && !current[..current.len()-1].ends_with('\\') {
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
                // Rest of line is comment
                while let Some(cc) = chars.next() {
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
    if word.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
        && word.chars().nth(1).map(|c| c.is_lowercase()).unwrap_or(false) {
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
        Language::Json | Language::Yaml | Language::Toml | Language::Html
        | Language::Xml | Language::Css | Language::Markdown | Language::Plain => false,
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
        Language::Json | Language::Yaml | Language::Toml | Language::Html
        | Language::Xml | Language::Css | Language::Markdown | Language::Bash
        | Language::Sql | Language::Plain => false,
    }
}

/// Check if a word is a built-in function in the given language.
fn is_function(word: &str, lang: Language) -> bool {
    match lang {
        Language::Rust => false, // Rust uses :: for methods, but we could add common ones
        Language::Python => PYTHON_FUNCTIONS.contains(&word),
        Language::JavaScript | Language::TypeScript => JS_FUNCTIONS.contains(&word),
        Language::Go => GO_FUNCTIONS.contains(&word),
        Language::Java => JAVA_FUNCTIONS.contains(&word),
        Language::C | Language::Cpp => false,
        Language::Json | Language::Yaml | Language::Toml | Language::Html
        | Language::Xml | Language::Css | Language::Markdown | Language::Bash
        | Language::Sql | Language::Plain => false,
    }
}

// Keyword sets
const RUST_KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else",
    "enum", "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop",
    "match", "mod", "move", "mut", "pub", "ref", "return", "self", "Self", "static",
    "struct", "super", "trait", "true", "type", "unsafe", "use", "where", "while",
];

const RUST_TYPES: &[&str] = &[
    "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128",
    "usize", "f32", "f64", "bool", "char", "str", "String", "Vec", "Option", "Result",
    "Box", "Rc", "Arc", "Cell", "RefCell", "HashMap", "HashSet", "BTreeMap", "BTreeSet",
    "Duration", "Instant", "SystemTime", "Path", "PathBuf", "OsString", "CString",
    "String", "Vec", "Array", "Slice", "Iterator", "IntoIterator", "From", "Into",
];

const PYTHON_KEYWORDS: &[&str] = &[
    "False", "None", "True", "and", "as", "assert", "async", "await", "break",
    "class", "continue", "def", "del", "elif", "else", "except", "finally",
    "for", "from", "global", "if", "import", "in", "is", "lambda", "nonlocal",
    "not", "or", "pass", "raise", "return", "try", "while", "with", "yield",
    "match", "case",
];

const PYTHON_BUILTINS: &[&str] = &[
    "int", "float", "str", "bool", "list", "dict", "set", "tuple", "bytes",
    "bytearray", "range", "slice", "property", "classmethod", "staticmethod",
    "object", "type", "super", "vars", "dir", "hash", "len", "abs", "all", "any",
    "bin", "bool", "chr", "ord", "hex", "oct", "pow", "round", "sorted", "sum",
    "min", "max", "divmod", "enumerate", "filter", "map", "next", "reversed",
    "zip", "isinstance", "issubclass", "callable", "hasattr", "getattr", "setattr",
    "delattr", "repr", "ascii", "format", "print", "input", "open", "compile",
    "exec", "eval", "breakpoint", "exit", "quit", "help", "copyright", "credits",
    "license", "credits",
];

const PYTHON_FUNCTIONS: &[&str] = &[
    "print", "len", "range", "str", "int", "float", "list", "dict", "set", "tuple",
    "open", "input", "map", "filter", "zip", "enumerate", "sorted", "reversed",
    "sum", "min", "max", "abs", "any", "all", "isinstance", "hasattr", "getattr",
    "setattr", "repr", "format", "round", "pow", "divmod", "bin", "hex", "oct",
    "chr", "ord", "slice", "super", "type", "vars", "dir", "hash", "callable",
];

const JS_KEYWORDS: &[&str] = &[
    "async", "await", "break", "case", "catch", "class", "const", "continue",
    "debugger", "default", "delete", "do", "else", "export", "extends", "false",
    "finally", "for", "function", "if", "import", "in", "instanceof", "let", "new",
    "null", "return", "static", "super", "switch", "this", "throw", "true", "try",
    "typeof", "undefined", "var", "void", "while", "with", "yield", "of", "get", "set",
];

const JS_TYPES: &[&str] = &[
    "Array", "Boolean", "Date", "Error", "Function", "JSON", "Map", "Math", "Number",
    "Object", "Promise", "Proxy", "Reflect", "RegExp", "Set", "String", "Symbol",
    "WeakMap", "WeakSet", "console", "document", "window", "globalThis",
    "ArrayBuffer", "DataView", "Float32Array", "Float64Array", "Int8Array",
    "Int16Array", "Int32Array", "Uint8Array", "Uint16Array", "Uint32Array",
    "BigInt", "BigInt64Array", "BigUint64Array",
];

const JS_FUNCTIONS: &[&str] = &[
    "log", "warn", "error", "info", "debug", "trace", "table", "assert", "clear",
    "count", "group", "groupEnd", "time", "timeEnd", "timeLog", "dir", "dirxml",
    "prototype", "constructor", "toString", "valueOf", "hasOwnProperty", "isPrototypeOf",
    "propertyIsEnumerable", "toLocaleString", "call", "apply", "bind", "then", "catch",
    "finally", "resolve", "reject", "all", "race", "allSettled", "any",
];

const GO_KEYWORDS: &[&str] = &[
    "break", "case", "chan", "const", "continue", "default", "defer", "else",
    "fallthrough", "for", "func", "go", "goto", "if", "import", "interface", "map",
    "package", "range", "return", "select", "struct", "switch", "type", "var",
    "nil", "true", "false", "iota",
];

const GO_TYPES: &[&str] = &[
    "bool", "byte", "complex64", "complex128", "error", "float32", "float64",
    "int", "int8", "int16", "int32", "int64", "rune", "string", "uint",
    "uint8", "uint16", "uint32", "uint64", "uintptr", "any",
    "chan", "func", "interface", "map", "pointer", "slice", "struct",
];

const GO_FUNCTIONS: &[&str] = &[
    "append", "cap", "close", "complex", "copy", "delete", "imag", "len", "make",
    "new", "panic", "print", "println", "real", "recover", "clear", "min", "max",
    "fmt", "strings", "io", "os", "bufio", "encoding", "net", "http",
];

const JAVA_KEYWORDS: &[&str] = &[
    "abstract", "assert", "boolean", "break", "byte", "case", "catch", "char",
    "class", "const", "continue", "default", "do", "double", "else", "enum",
    "extends", "final", "finally", "float", "for", "goto", "if", "implements",
    "import", "instanceof", "int", "interface", "long", "native", "new", "package",
    "private", "protected", "public", "return", "short", "static", "strictfp",
    "super", "switch", "synchronized", "this", "throw", "throws", "transient",
    "try", "void", "volatile", "while", "true", "false", "null",
];

const JAVA_TYPES: &[&str] = &[
    "Boolean", "Byte", "Character", "Double", "Float", "Integer", "Long", "Object",
    "Short", "String", "Void", "Class", "Enum", "Record", "StringBuilder", "StringBuffer",
    "ArrayList", "HashMap", "HashSet", "LinkedList", "TreeMap", "TreeSet", "HashMap",
    "Optional", "Stream", "Collection", "List", "Set", "Map", "Iterator", "Iterable",
    "Comparable", "Comparator", "Runnable", "Thread", "Exception", "RuntimeException",
];

const JAVA_FUNCTIONS: &[&str] = &[
    "println", "print", "printf", "toString", "hashCode", "equals", "getClass",
    "notify", "notifyAll", "wait", "getName", "setName", "start", "run", "sleep",
    "join", "yield", "currentThread", "arraycopy", "sort", "binarySearch",
    "copyOf", "copyOfRange", "fill", "equals", "deepEquals", "toHexString",
];

const C_KEYWORDS: &[&str] = &[
    "auto", "break", "case", "char", "const", "continue", "default", "do", "double",
    "else", "enum", "extern", "float", "for", "goto", "if", "inline", "int", "long",
    "register", "restrict", "return", "short", "signed", "sizeof", "static", "struct",
    "switch", "typedef", "union", "unsigned", "void", "volatile", "while", "_Bool",
    "_Complex", "_Imaginary", "NULL", "true", "false",
];

const C_TYPES: &[&str] = &[
    "bool", "char", "double", "float", "int", "long", "short", "void", "size_t",
    "ptrdiff_t", "intptr_t", "uintptr_t", "FILE", "DIR", "struct", "union", "enum",
    "typedef", "unsigned", "signed", "const", "volatile", "restrict",
];

const SQL_KEYWORDS: &[&str] = &[
    "SELECT", "FROM", "WHERE", "INSERT", "UPDATE", "DELETE", "CREATE", "DROP",
    "ALTER", "TABLE", "INDEX", "VIEW", "DATABASE", "SCHEMA", "INTO", "VALUES",
    "SET", "AND", "OR", "NOT", "NULL", "IS", "IN", "LIKE", "BETWEEN", "JOIN",
    "LEFT", "RIGHT", "INNER", "OUTER", "FULL", "CROSS", "ON", "AS", "ORDER",
    "BY", "GROUP", "HAVING", "LIMIT", "OFFSET", "UNION", "ALL", "DISTINCT",
    "COUNT", "SUM", "AVG", "MIN", "MAX", "CASE", "WHEN", "THEN", "ELSE", "END",
    "PRIMARY", "KEY", "FOREIGN", "REFERENCES", "CONSTRAINT", "DEFAULT", "CHECK",
    "UNIQUE", "CASCADE", "RESTRICT", "TRANSACTION", "BEGIN", "COMMIT", "ROLLBACK",
    "SAVEPOINT", "EXPLAIN", "ANALYZE", "VACUUM", "PRAGMA",
];

const BASH_KEYWORDS: &[&str] = &[
    "if", "then", "else", "elif", "fi", "case", "esac", "for", "while", "until",
    "do", "done", "in", "function", "select", "time", "coproc", "return", "exit",
    "break", "continue", "local", "declare", "typeset", "readonly", "export",
    "unset", "shift", "source", "alias", "unalias", "set", "shopt", "trap",
    "eval", "exec", "true", "false",
];

/// Highlight code content with syntax tokens.
pub fn highlight_code(content: &str, lang: &str) -> Vec<Vec<SyntaxToken>> {
    let language = Language::from_fence(lang);
    content
        .lines()
        .map(|line| tokenize_line(line, language))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_keyword_highlighting() {
        let line = "fn main() {}";
        let tokens = tokenize_line(line, Language::Rust);
        let contents: Vec<_> = tokens.iter().map(|t| t.content.as_str()).collect();
        assert!(contents.contains(&"fn"), "Should have fn keyword");
        assert!(contents.contains(&"main"), "Should have function name");
        // The tokenizer preserves the line, check it has the brace
        let full: String = tokens.iter().map(|t| t.content.as_str()).collect();
        assert!(full.contains('{'), "Should have brace");
    }

    #[test]
    fn python_keyword_highlighting() {
        let line = "def hello():";
        let tokens = tokenize_line(line, Language::Python);
        let contents: Vec<_> = tokens.iter().map(|t| t.content.as_str()).collect();
        assert!(contents.contains(&"def"), "Should have def keyword");
        assert!(contents.contains(&"hello"), "Should have function name");
    }

    #[test]
    fn js_string_highlighting() {
        let line = r#"let msg = "hello";"#;
        let tokens = tokenize_line(line, Language::JavaScript);
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
        let tokens = tokenize_line(line, Language::JavaScript);
        let num_token = tokens.iter().find(|t| t.content == "42");
        assert!(num_token.is_some(), "Should have number token");
    }

    #[test]
    fn comment_highlighting() {
        let line = "// this is a comment";
        let tokens = tokenize_line(line, Language::Rust);
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
        let tokens = tokenize_line(rust_code, Language::Rust);
        assert!(!tokens.is_empty(), "Should have tokens");

        let python_code = "x = 42";
        let py_tokens = tokenize_line(python_code, Language::Python);
        assert!(!py_tokens.is_empty(), "Should have python tokens");
    }

    #[test]
    fn highlight_code_multiline() {
        let code = r#"fn main() {
    let x = 42;
}"#;
        let lines = highlight_code(code, "rust");
        assert_eq!(lines.len(), 3, "Should have 3 lines");
        assert!(!lines[0].is_empty(), "First line should have tokens");
    }

    #[test]
    fn sql_keyword_highlighting() {
        let line = "SELECT * FROM users";
        let tokens = tokenize_line(line, Language::Sql);
        let contents: Vec<_> = tokens.iter().map(|t| t.content.as_str()).collect();
        assert!(contents.contains(&"SELECT"), "Should have SELECT keyword");
        assert!(contents.contains(&"FROM"), "Should have FROM keyword");
    }

    #[test]
    fn go_keyword_highlighting() {
        let line = "package main";
        let tokens = tokenize_line(line, Language::Go);
        let contents: Vec<_> = tokens.iter().map(|t| t.content.as_str()).collect();
        assert!(contents.contains(&"package"), "Should have package keyword");
        assert!(contents.contains(&"main"), "Should have main identifier");
    }

    #[test]
    fn type_highlighting() {
        // Test that Rust types get colored
        let line = "let name: String = String::new();";
        let tokens = tokenize_line(line, Language::Rust);
        let type_token = tokens.iter().find(|t| t.content == "String");
        assert!(type_token.is_some(), "Should have String type token");
    }

    #[test]
    fn empty_line() {
        let tokens = tokenize_line("", Language::Rust);
        assert!(tokens.is_empty() || tokens.len() == 1, "Empty line should have minimal tokens");
    }
}
