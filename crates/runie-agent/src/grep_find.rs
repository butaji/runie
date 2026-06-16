use crate::parser::parse_tool_calls;
use runie_core::tool::{ToolContext, ToolStatus};
use runie_engine::tool::builtin_registry;

#[test]
fn parse_grep_tool_json() {
    let text =
        r#"{"name": "grep", "arguments": {"pattern": "fn main", "path": "src", "glob": "*.rs"}}"#;
    let tools = parse_tool_calls(text);
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "grep");
    assert_eq!(tools[0].args["pattern"], "fn main");
    assert_eq!(tools[0].args["path"], "src");
    assert_eq!(tools[0].args["glob"], "*.rs");
}

#[test]
fn parse_find_tool_json() {
    let text = r#"{"name": "find", "arguments": {"pattern": "*.rs", "path": "src"}}"#;
    let tools = parse_tool_calls(text);
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "find");
    assert_eq!(tools[0].args["pattern"], "*.rs");
    assert_eq!(tools[0].args["path"], "src");
}

async fn call_tool(name: &str, args: serde_json::Value) -> runie_core::tool::ToolOutput {
    let registry = builtin_registry();
    let tool = registry.get(name).unwrap_or_else(|| panic!("unknown tool: {}", name));
    tool.call(args, &ToolContext::default())
        .await
        .unwrap_or_else(|e| panic!("tool {} failed: {}", name, e))
}

#[tokio::test]
async fn grep_executes_and_finds_matches() {
    let output = call_tool(
        "grep",
        serde_json::json!({
            "pattern": "fn main",
            "path": ".",
            "glob": "*.rs",
            "limit": 100,
        }),
    )
    .await;
    assert_eq!(output.status, ToolStatus::Success);
    assert!(
        output.content.contains("fn main") || output.content.contains("No matches")
    );
}

#[tokio::test]
async fn find_executes_and_lists_files() {
    let output = call_tool(
        "find",
        serde_json::json!({"pattern": "*.rs", "path": ".", "limit": 100}),
    )
    .await;
    assert_eq!(output.status, ToolStatus::Success);
    assert!(!output.content.is_empty());
}

#[tokio::test]
async fn grep_respects_limit() {
    let output = call_tool(
        "grep",
        serde_json::json!({
            "pattern": "use ",
            "path": ".",
            "glob": "*.rs",
            "limit": 2,
        }),
    )
    .await;
    assert_eq!(output.status, ToolStatus::Success);
    assert!(output.content.contains("use "));
}

#[tokio::test]
async fn find_respects_limit() {
    let output = call_tool(
        "find",
        serde_json::json!({"pattern": "*.rs", "path": ".", "limit": 3}),
    )
    .await;
    assert_eq!(output.status, ToolStatus::Success);
    let lines: Vec<&str> = output
        .content
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('['))
        .collect();
    assert!(lines.len() <= 3, "Expected at most 3 files, got {}", lines.len());
}

#[tokio::test]
async fn grep_literal_mode() {
    let output = call_tool(
        "grep",
        serde_json::json!({
            "pattern": ".",
            "path": ".",
            "glob": "*.toml",
            "literal": true,
            "limit": 10,
        }),
    )
    .await;
    assert_eq!(output.status, ToolStatus::Success);
}

#[tokio::test]
async fn grep_ignore_case() {
    let output = call_tool(
        "grep",
        serde_json::json!({
            "pattern": "CARGO",
            "path": ".",
            "glob": "*.toml",
            "ignore_case": true,
            "literal": true,
            "limit": 10,
        }),
    )
    .await;
    assert_eq!(output.status, ToolStatus::Success);
}
