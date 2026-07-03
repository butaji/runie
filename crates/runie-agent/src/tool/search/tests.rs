//! Tests for the search tool.

use super::types::*;
use super::*;
use crate::tool::search::core::SearchInput;
use crate::tool::{ToolContext, ToolDef, ToolStatus};
use git2::Status as GitStatus;
use runie_core::location::{parse_search_query, Location};

#[test]
fn search_mode_from_str() {
    assert_eq!(SearchMode::from_str("files"), SearchMode::Files);
    assert_eq!(SearchMode::from_str("content"), SearchMode::Content);
    assert_eq!(SearchMode::from_str("mixed"), SearchMode::Mixed);
    assert_eq!(SearchMode::from_str("glob"), SearchMode::Glob);
    assert_eq!(SearchMode::from_str("unknown"), SearchMode::Files);
}

#[test]
fn search_tool_name() {
    assert_eq!(SearchTool::NAME, "search");
}

#[test]
fn search_tool_is_read_only() {
    const _: () = assert!(SearchTool::READ_ONLY, "search must be read-only");
}

#[test]
fn search_tool_no_approval_required() {
    const _: () = assert!(
        !SearchTool::REQUIRES_APPROVAL,
        "search must not require approval"
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
    let modified_status = GitStatus::WT_MODIFIED;
    let item =
        build_search_item("src/lib.rs".to_string(), Some(modified_status), 0.95);
    assert_eq!(item.path, "src/lib.rs");
    assert_eq!(item.score, 0.95);
    assert_eq!(item.git_status, Some("modified".to_string()));
    assert!(item.line.is_none());
    assert!(item.col.is_none());
    assert!(item.content.is_none());
}

#[test]
fn build_search_item_skips_clean_git_status() {
    let clean_status = GitStatus::empty();
    let item = build_search_item("src/main.rs".to_string(), Some(clean_status), 0.8);
    assert_eq!(item.git_status, None);
}

#[test]
fn build_search_item_skips_empty_git_status() {
    let item = build_search_item("src/main.rs".to_string(), None, 0.8);
    assert_eq!(item.git_status, None);
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

// ─────────────────────────────────────────────────────────────────────────────
// Query parser tests (replaces fff_search::QueryParser tests)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn query_parser_applies_glob_constraint() {
    let parsed = parse_search_query("*.rs");
    let has_glob = parsed.globs().eq(["*.rs"]);
    assert!(has_glob, "Expected glob constraint, got: {:?}", parsed.constraints);
}

#[test]
fn query_parser_applies_negation() {
    let parsed = parse_search_query("config !test/ !vendor/");
    assert_eq!(parsed.text, "config");
    assert!(parsed.negations().eq(["test/", "vendor/"]));
}

#[test]
fn query_parser_handles_git_status_filter() {
    let parsed = parse_search_query("git:modified");
    assert!(parsed.git_status_filters().eq(["modified"]));
}

#[test]
fn query_parser_handles_location_hint() {
    let parsed = parse_search_query("lib.rs:42");
    assert_eq!(parsed.text, "lib.rs");
    assert!(matches!(parsed.location, Some(Location::Line(n)) if n == 42));
}

#[test]
fn query_parser_handles_location_with_column() {
    let parsed = parse_search_query("lib.rs:42:5");
    assert!(matches!(
        parsed.location,
        Some(Location::Position { line: 42, col: 5 })
    ));
}

#[test]
fn query_parser_handles_mixed_query() {
    let parsed = parse_search_query("config yaml !test/ git:modified *.rs");
    assert_eq!(parsed.text, "config yaml");
    assert!(parsed.globs().eq(["*.rs"]));
    assert!(parsed.negations().eq(["test/"]));
    assert!(parsed.git_status_filters().eq(["modified"]));
}

#[tokio::test]
async fn search_tool_handles_uninitialized_indexer() {
    let ctx = ToolContext::default();
    let input = SearchInput {
        query: "test".to_string(),
        path: None,
        mode: None,
        limit: None,
    };
    let output = SearchTool::execute(input, &ctx).await;
    assert!(
        output.status == ToolStatus::Error
            || output.content.contains("not initialized")
            || output.content.contains("items"),
        "Got: {}",
        output.content
    );
}
