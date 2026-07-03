//! `find_definitions` tool — locate symbol definitions using content search.

#![allow(non_upper_case_globals)]

use crate::tool::constants::FIND_DEFINITIONS_DEFAULT_LIMIT;
use crate::tool::search::fff_helpers::{
    build_error_json, build_error_json_with_instant, with_search_index,
};
use crate::tool::{ToolContext, ToolOutput, ToolStatus};
use regex::Regex;
use runie_core::actors::fff_indexer::SearchIndex;
use runie_core::actors::FffSearchState;
use runie_core::tool::ToolDef;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use std::time::Instant;

/// Input parameters for find_definitions tool.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct FindDefinitionsInput {
    /// Symbol name or pattern to search for (e.g., 'MyStruct', 'handle_request')
    pub symbol: String,
    /// Optional glob pattern to restrict to file types (e.g., '*.rs', '**/*.go')
    #[serde(default)]
    pub glob: Option<String>,
    /// Root directory to search (default: current directory)
    #[serde(default)]
    pub path: Option<String>,
    /// Maximum number of results (default: 30)
    #[serde(default)]
    pub limit: Option<usize>,
}

/// Max file size for content indexing (2 MiB).
const MAX_FILE_SIZE: usize = 2 * 1024 * 1024;

/// A compiled regex pattern paired with its definition kind.
type PatternEntry = (&'static LazyLock<Regex>, &'static str);

/// Static table of regex patterns for definition detection, in priority order.
/// Patterns are checked sequentially; the first match determines the kind.
///
/// Covers Rust, Python, TypeScript/JS, Go, Ruby, Java, and C family languages.
/// For `impl<T>` blocks, we strip generics before matching to avoid false positives
/// on `<` in comparison operators.
static PATTERNS: &[PatternEntry] = &[
    // Rust: impl<T> blocks — strip generics first, then match `impl `
    (&IMPL_GENERIC, "impl"),
    // Rust
    (&RUST_STRUCT, "struct"),
    (&RUST_FN, "fn"),
    (&RUST_ENUM, "enum"),
    (&RUST_TRAIT, "trait"),
    (&RUST_IMPL, "impl"),
    // Python
    (&PY_CLASS, "class"),
    (&PY_DEF, "def"),
    // TypeScript / JavaScript
    (&TS_CLASS, "class"),
    (&TS_INTERFACE, "interface"),
    (&TS_TYPE, "type"),
    (&TS_ENUM, "enum"),
    // Go
    (&GO_STRUCT, "struct"),
    (&GO_FUNC, "fn"),
    (&GO_INTERFACE, "interface"),
    (&GO_TYPE, "type"),
    // Ruby
    (&RUBY_DEF, "def"),
    (&RUBY_CLASS, "class"),
    // Java
    (&JAVA_CLASS, "class"),
    (&JAVA_INTERFACE, "interface"),
    (&JAVA_ENUM, "enum"),
    // C / C++
    (&C_STRUCT, "struct"),
    (&C_ENUM, "enum"),
    (&C_TYPEDEF, "type"),
    // Shell / scripting
    (&SH_FUNC, "fn"),
];

// Individual compiled regex patterns for each language/construct combination.
// Rust vis patterns handle: pub | pub(crate) | pub(super) | crate | (none).
// Single-keyword patterns use \b to avoid matching keywords inside other language constructs
// (e.g., "type MyStruct struct {" should match Go struct, not Rust enum).
static RUST_STRUCT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:(?:pub(?:\s*\(\s*crate\s*\)|\s*\(\s*super\s*\))?|crate)\s+)?\bstruct\s+").unwrap()
});
static RUST_FN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:(?:pub(?:\s*\(\s*crate\s*\)|\s*\(\s*super\s*\))?|crate)\s+)?async\s+fn\b|^\s*(?:(?:pub(?:\s*\(\s*crate\s*\)|\s*\(\s*super\s*\))?|crate)\s+)?fn\b").unwrap()
});
static RUST_ENUM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(?:(?:pub(?:\s*\(\s*crate\s*\)|\s*\(\s*super\s*\))?|crate)\s+)?\benum\s+").unwrap());
static RUST_TRAIT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(?:(?:pub(?:\s*\(\s*crate\s*\)|\s*\(\s*super\s*\))?|crate)\s+)?\btrait\s+").unwrap());
static RUST_IMPL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(?:(?:pub(?:\s*\(\s*crate\s*\)|\s*\(\s*super\s*\))?|crate)\s+)?\bimpl\s+").unwrap());
static IMPL_GENERIC: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(?:(?:pub(?:\s*\(\s*crate\s*\)|\s*\(\s*super\s*\))?|crate)\s+)?\bimpl<[^>]+>\s+").unwrap());
static PY_CLASS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*\bclass\s").unwrap());
// Python `def`: matches the original starts_with("def ") / "pub def " / "async def " behavior.
static PY_DEF: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(?:pub\s+)?(?:async\s+)?def\s").unwrap());
static TS_CLASS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(?:export\s+)?(?:abstract\s+)?(?:pub\s+)?\bclass\s").unwrap());
static TS_INTERFACE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(?:export\s+)?\binterface\s").unwrap());
// TS_TYPE: requires "=" or "<" after the identifier to distinguish from Go's
// "type X struct {" pattern. Matches: "type Foo =", "type Foo<", "type Foo<T> =".
static TS_TYPE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(?:export\s+)?type\s+\w+.*(?:=|<)").unwrap());
static TS_ENUM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*\benum\s").unwrap());

static GO_STRUCT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*type\s+\w+\s+struct\s*\{").unwrap());
static GO_FUNC: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*func\s+(?:\([^)]+\)\s*)?").unwrap());
static GO_INTERFACE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*type\s+\w+\s+interface\s*\{").unwrap());
static GO_TYPE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*type\s+\w+\s*=").unwrap());
static RUBY_DEF: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*\bdef\s+(?:self\.)?").unwrap());
static RUBY_CLASS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*\bclass\s").unwrap());
static JAVA_CLASS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(?:public\s+)?(?:abstract\s+|final\s+)?class\s+").unwrap());
static JAVA_INTERFACE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(?:public\s+)?interface\s+").unwrap());
static JAVA_ENUM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(?:public\s+)?enum\s+").unwrap());
static C_STRUCT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(?:typedef\s+)?struct\s+").unwrap());
static C_ENUM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*enum\s+").unwrap());
static C_TYPEDEF: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*typedef\s+").unwrap());
static SH_FUNC: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*\w+\s*\(\)\s*\{").unwrap());

/// Detect the definition kind from the line text using a compiled regex table.
fn detect_kind(line: &str) -> &'static str {
    let t = line.trim();
    // Special-case `impl<` generics: strip the generic portion before matching.
    if let Some(pos) = t.find('<') {
        let stripped = &t[..pos];
        if RUST_IMPL.is_match(stripped) || stripped.starts_with("impl ") || stripped.starts_with("pub impl ") {
            return "impl";
        }
    }
    for (pattern, kind) in PATTERNS {
        if pattern.is_match(t) {
            return kind;
        }
    }
    "definition"
}

/// `find_definitions` tool.
pub struct FindDefinitionsTool;

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DefResult {
    path: String,
    line: u64,
    col: usize,
    kind: String,
    content: String,
}

impl ToolDef for FindDefinitionsTool {
    type Input = FindDefinitionsInput;

    const NAME: &'static str = "find_definitions";
    const DESCRIPTION: &'static str = "Find symbol definitions (struct, fn, class, def, impl, enum, trait, etc.) in the codebase.";
    const READ_ONLY: bool = true;
    const REQUIRES_APPROVAL: bool = false;

    async fn execute(input: Self::Input, _ctx: &ToolContext) -> ToolOutput {
        let start = Instant::now();
        let state = match FffSearchState::get() {
            Some(s) => s,
            None => {
                return build_error_json_with_instant(
                    "find_definitions",
                    serde_json::json!({ "symbol": input.symbol }),
                    "Search indexer not initialized",
                    "results",
                    false,
                    start,
                )
                .unwrap_or_else(|_| ToolOutput {
                    tool_name: "find_definitions".to_owned(),
                    tool_args: serde_json::json!({ "symbol": input.symbol }),
                    content: "Search indexer not initialized".to_owned(),
                    bytes_transferred: None,
                    duration: start.elapsed(),
                    status: ToolStatus::Error,
                })
            }
        };
        with_search_index(
            &state,
            input.symbol.clone(),
            start,
            build_find_def_not_initialized,
            |index| {
                Ok(search_definitions(
                    index,
                    &input.symbol,
                    &input.glob,
                    input.limit.unwrap_or(FIND_DEFINITIONS_DEFAULT_LIMIT),
                    start,
                ))
            },
        )
        .unwrap_or_else(|e| ToolOutput {
            tool_name: "find_definitions".to_owned(),
            tool_args: serde_json::json!({ "symbol": input.symbol }),
            content: format!("find_definitions error: {}", e),
            bytes_transferred: None,
            duration: start.elapsed(),
            status: ToolStatus::Error,
        })
    }
}

fn build_find_def_not_initialized(symbol: String, duration: std::time::Duration) -> ToolOutput {
    build_error_json(
        "find_definitions",
        serde_json::json!({ "symbol": symbol }),
        "Search indexer not initialized",
        "results",
        false,
        duration,
    )
}

fn search_definitions(
    index: &SearchIndex,
    symbol: &str,
    glob: &Option<String>,
    limit: usize,
    start: Instant,
) -> ToolOutput {
    let query_str = build_query(symbol, glob);
    let matches = index.grep(&query_str, MAX_FILE_SIZE, 5, limit);

    // Filter matches that look like definitions based on content analysis.
    let defs: Vec<DefResult> = matches
        .into_iter()
        .filter(|m| is_definition_line(&m.line_content))
        .map(|m| DefResult {
            path: m.path,
            line: m.line_number,
            col: m.col,
            kind: detect_kind(&m.line_content).to_owned(),
            content: m.line_content,
        })
        .take(limit)
        .collect();

    let indexed = FffSearchState::is_indexed();
    build_definitions_output(symbol, defs, indexed, start)
}

fn build_query(symbol: &str, glob: &Option<String>) -> String {
    match glob {
        Some(g) if !g.is_empty() => format!("{} {}", symbol, g),
        _ => symbol.to_owned(),
    }
}

/// Check if a line looks like a definition.
fn is_definition_line(line: &str) -> bool {
    detect_kind(line) != "definition"
}

fn build_definitions_output(
    symbol: &str,
    defs: Vec<DefResult>,
    indexed: bool,
    start: Instant,
) -> ToolOutput {
    ToolOutput {
        tool_name: "find_definitions".to_owned(),
        tool_args: serde_json::json!({ "symbol": symbol }),
        content: serde_json::to_string_pretty(&serde_json::json!({
            "results": defs,
            "total": defs.len(),
            "indexed": indexed,
        }))
        .unwrap_or_default(),
        bytes_transferred: None,
        duration: start.elapsed(),
        status: ToolStatus::Success,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_kind_struct() {
        assert_eq!(detect_kind("pub struct MyStruct {"), "struct");
        assert_eq!(detect_kind("struct Inner"), "struct");
        assert_eq!(detect_kind("pub(crate) struct Foo"), "struct");
    }

    #[test]
    fn detect_kind_fn() {
        assert_eq!(detect_kind("fn handle_request() {"), "fn");
        assert_eq!(detect_kind("pub fn main() {"), "fn");
        assert_eq!(detect_kind("async fn spawn_task() {"), "fn");
        assert_eq!(detect_kind("pub async fn process() {"), "fn");
    }

    #[test]
    fn detect_kind_enum() {
        assert_eq!(detect_kind("pub enum Color {"), "enum");
        assert_eq!(detect_kind("enum Status {"), "enum");
    }

    #[test]
    fn detect_kind_trait() {
        assert_eq!(detect_kind("pub trait Serialize {"), "trait");
        assert_eq!(detect_kind("trait Debug {"), "trait");
    }

    #[test]
    fn detect_kind_impl() {
        assert_eq!(detect_kind("impl MyTrait for Foo {"), "impl");
        assert_eq!(detect_kind("impl<T> Vec<T> {"), "impl");
    }

    #[test]
    fn detect_kind_class() {
        assert_eq!(detect_kind("pub class MyClass:"), "class");
        assert_eq!(detect_kind("class Counter:"), "class");
    }

    #[test]
    fn detect_kind_def() {
        assert_eq!(detect_kind("def process_item(self, x):"), "def");
        assert_eq!(detect_kind("pub def initialize():"), "def");
    }

    #[test]
    fn detect_kind_fallback() {
        // A line that doesn't match any known kind.
        assert_eq!(detect_kind("  let x = 42;"), "definition");
    }

    #[test]
    fn detect_kind_go() {
        assert_eq!(detect_kind("type MyStruct struct {"), "struct");
        assert_eq!(detect_kind("func (m *MyStruct) Do() {"), "fn");
        assert_eq!(detect_kind("func init() {"), "fn");
        assert_eq!(detect_kind("type Reader interface {"), "interface");
        assert_eq!(detect_kind("type Alias = int"), "type");
    }

    #[test]
    fn detect_kind_java() {
        assert_eq!(detect_kind("public class Main {"), "class");
        assert_eq!(detect_kind("public interface Runnable {"), "interface");
        assert_eq!(detect_kind("public enum Status {"), "enum");
        assert_eq!(detect_kind("abstract class Base {"), "class");
    }

    #[test]
    fn detect_kind_typescript() {
        assert_eq!(detect_kind("export class Service {"), "class");
        assert_eq!(detect_kind("export interface Config {"), "interface");
        assert_eq!(detect_kind("export type Result = string | number"), "type");
        assert_eq!(detect_kind("enum Color { Red, Green }"), "enum");
        assert_eq!(detect_kind("abstract class Base {"), "class");
        assert_eq!(detect_kind("pub class API {"), "class"); // TypeScript pub modifier
    }

    #[test]
    fn detect_kind_ruby() {
        assert_eq!(detect_kind("class Counter:"), "class");
        assert_eq!(detect_kind("def initialize:"), "def");
        assert_eq!(detect_kind("def self.factory:"), "def");
    }

    #[test]
    fn detect_kind_c() {
        assert_eq!(detect_kind("struct Point { double x; double y; };"), "struct");
        assert_eq!(detect_kind("typedef struct {} Node;"), "struct");
        assert_eq!(detect_kind("enum Color { RED, GREEN };"), "enum");
        assert_eq!(detect_kind("typedef int my_int;"), "type");
    }

    #[test]
    fn detect_kind_not_false_positive() {
        // Ensure we don't match similar-looking but non-definition lines.
        assert_eq!(detect_kind("fnord();"), "definition"); // fnord, not fn
        assert_eq!(detect_kind("const fnord = 1;"), "definition"); // fnord
        assert_eq!(detect_kind("structuring data..."), "definition"); // contains but not starts with
        assert_eq!(detect_kind("defined = True"), "definition"); // defined, not def
        assert_eq!(detect_kind("let className = 'foo';"), "definition"); // className
    }

    #[test]
    fn detect_kind_rust_crate_vis() {
        // Rust crate visibility variants.
        assert_eq!(detect_kind("pub(crate) struct Internal;"), "struct");
        assert_eq!(detect_kind("crate struct CrateVisible;"), "struct");
        assert_eq!(detect_kind("pub(super) struct Outer;"), "struct");
    }

    #[test]
    fn detect_kind_impl_generic() {
        // impl<T> with generics.
        assert_eq!(detect_kind("impl<T> Vec<T> {"), "impl");
        assert_eq!(detect_kind("impl<T, U> Pair<T, U> {"), "impl");
        assert_eq!(detect_kind("impl<T> Iterator for Counter<T> {"), "impl");
    }

    #[test]
    fn is_definition_line_fn() {
        assert!(is_definition_line("fn foo() {}"));
        assert!(is_definition_line("pub async fn bar() {}"));
        assert!(!is_definition_line("foo();"));
        assert!(!is_definition_line("// fn bar() {}"));
    }

    #[test]
    fn is_definition_line_struct() {
        assert!(is_definition_line("pub struct MyStruct {"));
        assert!(!is_definition_line("let s = MyStruct {};"));
    }

    #[test]
    fn find_definitions_tool_name() {
        assert_eq!(FindDefinitionsTool::NAME, "find_definitions");
    }

    #[test]
    fn find_definitions_tool_is_read_only() {
        assert!(FindDefinitionsTool::READ_ONLY);
    }

    #[test]
    fn find_definitions_tool_no_approval() {
        assert!(!FindDefinitionsTool::REQUIRES_APPROVAL);
    }

    #[test]
    fn find_definitions_tool_schema() {
        let schema = runie_core::tool::generate_schema::<FindDefinitionsInput>();
        assert!(schema.get("properties").is_some());
        let obj = schema.as_object().unwrap();
        let props = obj.get("properties").unwrap().as_object().unwrap();
        assert!(props.contains_key("symbol"));
        assert!(props.contains_key("glob"));
        assert!(props.contains_key("path"));
        assert!(props.contains_key("limit"));
    }

    #[tokio::test]
    async fn find_definitions_uninitialized_returns_error() {
        let input = FindDefinitionsInput {
            symbol: "Foo_xyz_nonexistent".to_string(),
            glob: None,
            path: None,
            limit: None,
        };
        let ctx = ToolContext::default();
        let output = FindDefinitionsTool::execute(input, &ctx).await;
        assert!(
            output.status == ToolStatus::Error
                || output.content.contains("not initialized")
                // Parallel tests may initialize the global indexer state; an empty
                // result for a non-existent symbol is still graceful behavior.
                || output.content.contains("\"total\": 0"),
            "Got: {}",
            output.content
        );
    }
}
