//! Keyword definitions for syntax highlighting

use std::fmt;
use std::hash::Hash;
use std::str;

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

// Keyword sets
pub const RUST_KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else",
    "enum", "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop",
    "match", "mod", "move", "mut", "pub", "ref", "return", "self", "Self", "static",
    "struct", "super", "trait", "true", "type", "unsafe", "use", "where", "while",
];

pub const RUST_TYPES: &[&str] = &[
    "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128",
    "usize", "f32", "f64", "bool", "char", "str", "String", "Vec", "Option", "Result",
    "Box", "Rc", "Arc", "Cell", "RefCell", "HashMap", "HashSet", "BTreeMap", "BTreeSet",
    "Duration", "Instant", "SystemTime", "Path", "PathBuf", "OsString", "CString",
    "String", "Vec", "Array", "Slice", "Iterator", "IntoIterator", "From", "Into",
];

pub const PYTHON_KEYWORDS: &[&str] = &[
    "False", "None", "True", "and", "as", "assert", "async", "await", "break",
    "class", "continue", "def", "del", "elif", "else", "except", "finally",
    "for", "from", "global", "if", "import", "in", "is", "lambda", "nonlocal",
    "not", "or", "pass", "raise", "return", "try", "while", "with", "yield",
    "match", "case",
];

pub const PYTHON_BUILTINS: &[&str] = &[
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

pub const PYTHON_FUNCTIONS: &[&str] = &[
    "print", "len", "range", "str", "int", "float", "list", "dict", "set", "tuple",
    "open", "input", "map", "filter", "zip", "enumerate", "sorted", "reversed",
    "sum", "min", "max", "abs", "any", "all", "isinstance", "hasattr", "getattr",
    "setattr", "repr", "format", "round", "pow", "divmod", "bin", "hex", "oct",
    "chr", "ord", "slice", "super", "type", "vars", "dir", "hash", "callable",
];

pub const JS_KEYWORDS: &[&str] = &[
    "async", "await", "break", "case", "catch", "class", "const", "continue",
    "debugger", "default", "delete", "do", "else", "export", "extends", "false",
    "finally", "for", "function", "if", "import", "in", "instanceof", "let", "new",
    "null", "return", "static", "super", "switch", "this", "throw", "true", "try",
    "typeof", "undefined", "var", "void", "while", "with", "yield", "of", "get", "set",
];

pub const JS_TYPES: &[&str] = &[
    "Array", "Boolean", "Date", "Error", "Function", "JSON", "Map", "Math", "Number",
    "Object", "Promise", "Proxy", "Reflect", "RegExp", "Set", "String", "Symbol",
    "WeakMap", "WeakSet", "console", "document", "window", "globalThis",
    "ArrayBuffer", "DataView", "Float32Array", "Float64Array", "Int8Array",
    "Int16Array", "Int32Array", "Uint8Array", "Uint16Array", "Uint32Array",
    "BigInt", "BigInt64Array", "BigUint64Array",
];

pub const JS_FUNCTIONS: &[&str] = &[
    "log", "warn", "error", "info", "debug", "trace", "table", "assert", "clear",
    "count", "group", "groupEnd", "time", "timeEnd", "timeLog", "dir", "dirxml",
    "prototype", "constructor", "toString", "valueOf", "hasOwnProperty", "isPrototypeOf",
    "propertyIsEnumerable", "toLocaleString", "call", "apply", "bind", "then", "catch",
    "finally", "resolve", "reject", "all", "race", "allSettled", "any",
];

pub const GO_KEYWORDS: &[&str] = &[
    "break", "case", "chan", "const", "continue", "default", "defer", "else",
    "fallthrough", "for", "func", "go", "goto", "if", "import", "interface", "map",
    "package", "range", "return", "select", "struct", "switch", "type", "var",
    "nil", "true", "false", "iota",
];

pub const GO_TYPES: &[&str] = &[
    "bool", "byte", "complex64", "complex128", "error", "float32", "float64",
    "int", "int8", "int16", "int32", "int64", "rune", "string", "uint",
    "uint8", "uint16", "uint32", "uint64", "uintptr", "any",
    "chan", "func", "interface", "map", "pointer", "slice", "struct",
];

pub const GO_FUNCTIONS: &[&str] = &[
    "append", "cap", "close", "complex", "copy", "delete", "imag", "len", "make",
    "new", "panic", "print", "println", "real", "recover", "clear", "min", "max",
    "fmt", "strings", "io", "os", "bufio", "encoding", "net", "http",
];

pub const JAVA_KEYWORDS: &[&str] = &[
    "abstract", "assert", "boolean", "break", "byte", "case", "catch", "char",
    "class", "const", "continue", "default", "do", "double", "else", "enum",
    "extends", "final", "finally", "float", "for", "goto", "if", "implements",
    "import", "instanceof", "int", "interface", "long", "native", "new", "package",
    "private", "protected", "public", "return", "short", "static", "strictfp",
    "super", "switch", "synchronized", "this", "throw", "throws", "transient",
    "try", "void", "volatile", "while", "true", "false", "null",
];

pub const JAVA_TYPES: &[&str] = &[
    "Boolean", "Byte", "Character", "Double", "Float", "Integer", "Long", "Object",
    "Short", "String", "Void", "Class", "Enum", "Record", "StringBuilder", "StringBuffer",
    "ArrayList", "HashMap", "HashSet", "LinkedList", "TreeMap", "TreeSet", "HashMap",
    "Optional", "Stream", "Collection", "List", "Set", "Map", "Iterator", "Iterable",
    "Comparable", "Comparator", "Runnable", "Thread", "Exception", "RuntimeException",
];

pub const JAVA_FUNCTIONS: &[&str] = &[
    "println", "print", "printf", "toString", "hashCode", "equals", "getClass",
    "notify", "notifyAll", "wait", "getName", "setName", "start", "run", "sleep",
    "join", "yield", "currentThread", "arraycopy", "sort", "binarySearch",
    "copyOf", "copyOfRange", "fill", "equals", "deepEquals", "toHexString",
];

pub const C_KEYWORDS: &[&str] = &[
    "auto", "break", "case", "char", "const", "continue", "default", "do", "double",
    "else", "enum", "extern", "float", "for", "goto", "if", "inline", "int", "long",
    "register", "restrict", "return", "short", "signed", "sizeof", "static", "struct",
    "switch", "typedef", "union", "unsigned", "void", "volatile", "while", "_Bool",
    "_Complex", "_Imaginary", "NULL", "true", "false",
];

pub const C_TYPES: &[&str] = &[
    "bool", "char", "double", "float", "int", "long", "short", "void", "size_t",
    "ptrdiff_t", "intptr_t", "uintptr_t", "FILE", "DIR", "struct", "union", "enum",
    "typedef", "unsigned", "signed", "const", "volatile", "restrict",
];

pub const SQL_KEYWORDS: &[&str] = &[
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

pub const BASH_KEYWORDS: &[&str] = &[
    "if", "then", "else", "elif", "fi", "case", "esac", "for", "while", "until",
    "do", "done", "in", "function", "select", "time", "coproc", "return", "exit",
    "break", "continue", "local", "declare", "typeset", "readonly", "export",
    "unset", "shift", "source", "alias", "unalias", "set", "shopt", "trap",
    "eval", "exec", "true", "false",
];
