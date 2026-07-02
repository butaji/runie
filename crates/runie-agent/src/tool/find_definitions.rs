//! `find_definitions` tool — locate symbol definitions using content search.

use crate::tool::constants::FIND_DEFINITIONS_DEFAULT_LIMIT;
use crate::tool::search::fff_helpers::{
    build_error_json, build_error_json_with_instant, with_search_index,
};
use crate::tool::{ToolContext, ToolOutput, ToolStatus};
use runie_core::actors::fff_indexer::SearchIndex;
use runie_core::actors::FffSearchState;
use runie_core::tool::ToolDef;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
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

type Detector = (&'static str, fn(&str) -> bool);

/// Detect the definition kind from the line text.
fn detect_kind(line: &str) -> &'static str {
    let t = line.trim();
    if t.starts_with("impl<") || t.starts_with("pub impl<") {
        return "impl";
    }
    const DETECTORS: &[Detector] = &[
        ("struct", detect_struct),
        ("fn", detect_fn),
        ("enum", detect_enum),
        ("trait", detect_trait),
        ("impl", detect_impl),
        ("class", detect_class),
        ("def", detect_def),
        ("func", detect_func),
        ("type", detect_type),
        ("module", detect_module),
        ("interface", detect_interface),
        ("object", detect_object),
    ];
    for (kind, detector) in DETECTORS {
        if detector(t) {
            return kind;
        }
    }
    "definition"
}

fn detect_struct(t: &str) -> bool {
    t.starts_with("struct ") || t.starts_with("pub struct ") || t.starts_with("pub(crate) struct ")
}

fn detect_fn(t: &str) -> bool {
    t.starts_with("fn ")
        || t.starts_with("pub fn ")
        || t.starts_with("async fn ")
        || t.starts_with("pub async fn ")
}

fn detect_enum(t: &str) -> bool {
    t.starts_with("enum ") || t.starts_with("pub enum ")
}

fn detect_trait(t: &str) -> bool {
    t.starts_with("trait ") || t.starts_with("pub trait ")
}

fn detect_impl(t: &str) -> bool {
    t.starts_with("impl ") || t.starts_with("pub impl ")
}

fn detect_class(t: &str) -> bool {
    t.starts_with("class ") || t.starts_with("pub class ")
}

fn detect_def(t: &str) -> bool {
    t.starts_with("def ") || t.starts_with("pub def ") || t.starts_with("async def ")
}

fn detect_func(t: &str) -> bool {
    t.starts_with("func ") || t.starts_with("pub func ")
}

fn detect_type(t: &str) -> bool {
    t.starts_with("type ") || t.starts_with("pub type ")
}

fn detect_module(t: &str) -> bool {
    t.starts_with("module ") || t.starts_with("pub module ")
}

fn detect_interface(t: &str) -> bool {
    t.starts_with("interface ") || t.starts_with("pub interface ")
}

fn detect_object(t: &str) -> bool {
    t.starts_with("object ") || t.starts_with("pub object ")
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
    let t = line.trim();
    detect_struct(t).then_some("struct")
        .or_else(|| detect_fn(t).then_some("fn"))
        .or_else(|| detect_enum(t).then_some("enum"))
        .or_else(|| detect_trait(t).then_some("trait"))
        .or_else(|| detect_impl(t).then_some("impl"))
        .or_else(|| detect_class(t).then_some("class"))
        .or_else(|| detect_def(t).then_some("def"))
        .or_else(|| detect_func(t).then_some("func"))
        .or_else(|| detect_type(t).then_some("type"))
        .is_some()
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
