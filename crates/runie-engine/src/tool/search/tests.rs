use super::types::*;
use super::*;
use crate::tool::{Tool, ToolContext, ToolStatus};
use fff_search::{Constraint, Location, QueryParser};
use git2::Status as GitStatus;

#[test]
fn search_mode_from_str() {
    assert_eq!(SearchMode::from_str("files"), SearchMode::Files);
    assert_eq!(SearchMode::from_str("content"), SearchMode::Content);
    assert_eq!(SearchMode::from_str("mixed"), SearchMode::Mixed);
    assert_eq!(SearchMode::from_str("glob"), SearchMode::Glob);
    assert_eq!(SearchMode::from_str("unknown"), SearchMode::Files);
}

#[test]
fn search_tool_schema_includes_glob_mode() {
    let tool = SearchTool;
    let schema = tool.input_schema();
    let mode = &schema["properties"]["mode"];
    let variants = mode["enum"].as_array().unwrap();
    assert!(
        variants.iter().any(|v| v == "glob"),
        "glob should be in mode enum"
    );
}

#[test]
fn search_tool_schema_has_examples() {
    let tool = SearchTool;
    let schema = tool.input_schema();
    let examples = schema["properties"]["query"]["examples"]
        .as_array()
        .unwrap();
    let has_glob = examples
        .iter()
        .any(|e| e.as_str().unwrap_or("").contains('*'));
    let has_negation = examples
        .iter()
        .any(|e| e.as_str().unwrap_or("").contains('!'));
    let has_git = examples
        .iter()
        .any(|e| e.as_str().unwrap_or("").starts_with("git:"));
    let has_location = examples
        .iter()
        .any(|e| e.as_str().unwrap_or("").contains(':'));
    assert!(has_glob, "should have glob example");
    assert!(has_negation, "should have negation example");
    assert!(has_git, "should have git filter example");
    assert!(has_location, "should have location example");
}

#[test]
fn search_tool_schema_documents_git_filter() {
    let tool = SearchTool;
    let desc = tool.description();
    assert!(
        desc.contains("git:"),
        "description should mention git: filter"
    );
}

#[test]
fn search_item_has_git_status_field() {
    let item = SearchItem {
        path: "src/lib.rs".to_string(),
        line: None,
        col: None,
        content: None,
        score: 1.0,
        git_status: Some("modified".to_string()),
    };
    let json = serde_json::to_string(&item).unwrap();
    assert!(
        json.contains("gitStatus"),
        "SearchItem should serialize gitStatus field"
    );
}

#[test]
fn build_search_item_creates_valid_item() {
    let status = GitStatus::from_bits_truncate(1 << 1); // WT_MODIFIED
    let item = build_search_item("src/lib.rs".to_string(), Some(status), 0.95);
    assert_eq!(item.path, "src/lib.rs");
    assert_eq!(item.score, 0.95);
    assert_eq!(item.git_status, Some("modified".to_string()));
    assert!(item.line.is_none());
    assert!(item.col.is_none());
    assert!(item.content.is_none());
}

#[test]
fn build_search_item_skips_empty_git_status() {
    let item = build_search_item("src/main.rs".to_string(), None, 0.8);
    assert_eq!(item.git_status, None);
}

#[test]
fn search_tool_schema() {
    let tool = SearchTool;
    let schema = tool.input_schema();
    assert!(schema.get("properties").is_some());
    let props = schema["properties"].as_object().unwrap();
    assert!(props.contains_key("query"));
    assert!(props.contains_key("mode"));
    assert!(props.contains_key("path"));
    assert!(props.contains_key("limit"));
}

#[test]
fn search_tool_name() {
    assert_eq!(SearchTool.name(), "search");
}

#[test]
fn search_tool_is_read_only() {
    assert!(SearchTool.is_read_only());
}

#[test]
fn search_tool_no_approval_required() {
    let input = serde_json::json!({"query": "test"});
    assert!(!SearchTool.requires_approval(&input));
}

#[test]
fn format_git_status_modified() {
    let status = GitStatus::from_bits_truncate(1 << 1);
    assert_eq!(format_git_status(status), "modified");
}

#[test]
fn format_git_status_untracked() {
    let status = GitStatus::from_bits_truncate(1 << 7);
    assert_eq!(format_git_status(status), "untracked");
}

#[test]
fn search_result_serialization() {
    let result = SearchResult {
        items: vec![SearchItem {
            path: "src/lib.rs".to_string(),
            line: Some(42),
            col: Some(5),
            content: Some("fn example() {}".to_string()),
            score: 0.95,
            git_status: Some("modified".to_string()),
        }],
        total: 1,
        indexed: true,
    };
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("src/lib.rs"));
    assert!(json.contains("42"));
    assert!(json.contains("modified"));
}

#[test]
fn query_parser_applies_glob_constraint() {
    let parsed = QueryParser::default().parse("*.rs");
    let has_glob = parsed
        .constraints
        .iter()
        .any(|c| matches!(c, Constraint::Glob(_) | Constraint::Extension(_)));
    assert!(
        has_glob,
        "Expected glob constraint, got: {:?}",
        parsed.constraints
    );
}

#[test]
fn query_parser_applies_negation() {
    let parsed = QueryParser::default().parse("config !test/ !vendor/");
    let has_negation = parsed
        .constraints
        .iter()
        .any(|c| matches!(c, Constraint::Not(_)));
    assert!(
        has_negation,
        "Expected Not constraint, got: {:?}",
        parsed.constraints
    );
}

#[test]
fn query_parser_handles_git_status_filter() {
    let parsed = QueryParser::default().parse("git:modified");
    let has_git = parsed
        .constraints
        .iter()
        .any(|c| matches!(c, Constraint::GitStatus(_)));
    assert!(
        has_git,
        "Expected git status constraint, got: {:?}",
        parsed.constraints
    );
}

#[test]
fn query_parser_handles_location_hint() {
    let parsed = QueryParser::default().parse("lib.rs:42");
    assert!(
        parsed.location.is_some(),
        "Expected location, got: {:?}",
        parsed.location
    );
    assert!(matches!(
        parsed.location.unwrap(),
        Location::Line(n) if n == 42
    ));
}

#[test]
fn query_parser_handles_location_with_column() {
    let parsed = QueryParser::default().parse("lib.rs:42:5");
    assert!(parsed.location.is_some(), "Expected location");
    assert!(matches!(
        parsed.location.unwrap(),
        Location::Position { line: 42, col: 5 }
    ));
}

#[test]
fn query_parser_handles_mixed_query() {
    let parsed = QueryParser::default().parse("config yaml !test/ git:modified *.rs");
    let has_glob = parsed
        .constraints
        .iter()
        .any(|c| matches!(c, Constraint::Glob(_) | Constraint::Extension(_)));
    let has_negation = parsed
        .constraints
        .iter()
        .any(|c| matches!(c, Constraint::Not(_)));
    let has_git = parsed
        .constraints
        .iter()
        .any(|c| matches!(c, Constraint::GitStatus(_)));
    assert!(
        has_glob && has_negation && has_git,
        "Expected glob+negation+git, got: {:?}",
        parsed.constraints
    );
}

#[tokio::test]
async fn search_tool_handles_uninitialized_indexer() {
    let tool = SearchTool;
    let ctx = ToolContext::default();
    let input = serde_json::json!({"query": "test"});
    let output = tool.call(input, &ctx).await.unwrap();
    assert!(
        output.status == ToolStatus::Error
            || output.content.contains("not initialized")
            || output.content.contains("items"),
        "Got: {}",
        output.content
    );
}
