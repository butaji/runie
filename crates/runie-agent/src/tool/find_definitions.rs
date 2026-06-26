//! `find_definitions` tool — locate symbol definitions using FFF's classifier.

use crate::tool::search::fff_helpers::{
    build_error_json, build_error_json_with_instant, with_picker,
};
use crate::tool::{Tool, ToolContext, ToolOutput, ToolStatus};
use anyhow::Result;
use async_trait::async_trait;
use fff_search::{FilePicker, GrepMatch, GrepMode, GrepResult, GrepSearchOptions, QueryParser};
use runie_core::actors::FffSearchState;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
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

/// Default max results.
const DEFAULT_LIMIT: usize = 30;

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

#[async_trait]
impl Tool for FindDefinitionsTool {
    fn name(&self) -> &str { "find_definitions" }
    fn description(&self) -> &str {
        "Find symbol definitions (struct, fn, class, def, impl, enum, trait, etc.) in the codebase using FFF's definition classifier."
    }
    fn input_schema(&self) -> Value {
        runie_core::tool::generate_schema::<FindDefinitionsInput>()
    }
    fn is_read_only(&self) -> bool { true }
    fn requires_approval(&self, _input: &Value) -> bool { false }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolOutput> {
        let start = Instant::now();
        let typed: FindDefinitionsInput = serde_json::from_value(input)?;
        let state = match FffSearchState::get() {
            Some(s) => s,
            None => {
                return build_error_json_with_instant(
                    "find_definitions",
                    serde_json::json!({ "symbol": typed.symbol }),
                    "FFF indexer not initialized",
                    "results",
                    false,
                    start,
                )
            }
        };
        with_picker(
            &state,
            typed.symbol.clone(),
            start,
            build_find_def_lock_error,
            build_find_def_not_initialized,
            |picker| search_definitions(picker, &typed.symbol, &typed.glob, typed.limit.unwrap_or(DEFAULT_LIMIT), start),
        )
    }
}

fn build_find_def_lock_error(msg: String, duration: std::time::Duration) -> ToolOutput {
    build_error_json(
        "find_definitions",
        serde_json::json!({ "symbol": msg }),
        &msg,
        "results",
        false,
        duration,
    )
}

fn build_find_def_not_initialized(symbol: String, duration: std::time::Duration) -> ToolOutput {
    build_error_json(
        "find_definitions",
        serde_json::json!({ "symbol": symbol }),
        "FFF picker not initialized",
        "results",
        false,
        duration,
    )
}

fn search_definitions(
    picker: &FilePicker,
    symbol: &str,
    glob: &Option<String>,
    limit: usize,
    start: Instant,
) -> Result<ToolOutput> {
    let query_str = build_query(symbol, glob);
    let parsed = QueryParser::default().parse(&query_str);
    let results = picker.grep(
        &parsed,
        &GrepSearchOptions {
            max_file_size: fff_search::MAX_FFFILE_SIZE,
            max_matches_per_file: 5,
            smart_case: true,
            file_offset: 0,
            page_limit: limit,
            mode: GrepMode::Regex,
            time_budget_ms: 5000,
            before_context: 0,
            after_context: 0,
            classify_definitions: true,
            trim_whitespace: true,
            abort_signal: None,
        },
    );
    let defs = map_definition_results(picker, &results, limit);
    let indexed = FffSearchState::is_indexed();
    build_definitions_output(symbol, defs, indexed, start)
}

fn build_query(symbol: &str, glob: &Option<String>) -> String {
    match glob {
        Some(g) if !g.is_empty() => format!("{} {}", symbol, g),
        _ => symbol.to_owned(),
    }
}

fn map_definition_results(
    picker: &FilePicker,
    results: &GrepResult<'_>,
    limit: usize,
) -> Vec<DefResult> {
    results
        .matches
        .iter()
        .filter(|m| m.is_definition)
        .take(limit)
        .map(|m| build_def_result(picker, results, m))
        .collect()
}

fn build_def_result(picker: &FilePicker, results: &GrepResult<'_>, m: &GrepMatch) -> DefResult {
    let path = results
        .files
        .get(m.file_index)
        .map(|f| f.relative_path(picker))
        .unwrap_or_else(|| format!("<file {}>", m.file_index));
    let kind = detect_kind(&m.line_content);
    DefResult {
        path,
        line: m.line_number,
        col: m.col,
        kind: kind.to_owned(),
        content: m.line_content.clone(),
    }
}

fn build_definitions_output(
    symbol: &str,
    defs: Vec<DefResult>,
    indexed: bool,
    start: Instant,
) -> Result<ToolOutput> {
    Ok(ToolOutput {
        tool_name: "find_definitions".to_owned(),
        tool_args: serde_json::json!({ "symbol": symbol }),
        content: serde_json::to_string_pretty(&serde_json::json!({
            "results": defs,
            "total": defs.len(),
            "indexed": indexed,
        }))?,
        bytes_transferred: None,
        duration: start.elapsed(),
        status: ToolStatus::Success,
    })
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
    fn find_definitions_tool_name() {
        assert_eq!(FindDefinitionsTool.name(), "find_definitions");
    }

    #[test]
    fn find_definitions_tool_is_read_only() {
        assert!(FindDefinitionsTool.is_read_only());
    }

    #[test]
    fn find_definitions_tool_no_approval() {
        let input = serde_json::json!({"symbol": "Foo"});
        assert!(!FindDefinitionsTool.requires_approval(&input));
    }

    #[test]
    fn find_definitions_tool_schema() {
        let tool = FindDefinitionsTool;
        let schema = tool.input_schema();
        assert!(schema.get("properties").is_some());
        let obj = schema.as_object().unwrap();
        let props = obj.get("properties").unwrap().as_object().unwrap();
        assert!(props.contains_key("symbol"));
        assert!(props.contains_key("glob"));
        assert!(props.contains_key("path"));
        assert!(props.contains_key("limit"));
    }

    #[test]
    fn find_definitions_tool_description_mentions_classifier() {
        let desc = FindDefinitionsTool.description();
        assert!(
            desc.contains("definition classifier"),
            "description: {}",
            desc
        );
    }

    #[tokio::test]
    async fn find_definitions_uninitialized_returns_error() {
        let tool = FindDefinitionsTool;
        let ctx = ToolContext::default();
        let input = serde_json::json!({"symbol": "Foo_xyz_nonexistent"});
        let output = tool.call(input, &ctx).await.unwrap();
        assert!(
            output.status == ToolStatus::Error
                || output.content.contains("not initialized")
                // Parallel tests may initialize the global FFF state; an empty
                // result for a non-existent symbol is still graceful behavior.
                || output.content.contains("\"total\": 0"),
            "Got: {}",
            output.content
        );
    }
}
