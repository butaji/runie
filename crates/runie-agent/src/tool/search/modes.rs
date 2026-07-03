//! Search mode implementations backed by the runie search index.

use crate::tool::search::types::{
    build_search_item, SearchItem, SearchResult, DEFAULT_MAX_MATCHES,
};
use crate::tool::{ToolOutput, ToolStatus};
use runie_core::actors::fff_indexer::SearchIndex;
use runie_core::location::parse_search_query;
use runie_core::tool::truncate_output;
use std::time::Instant;

/// Max file size for content indexing (matches MAX_FILE_SIZE in the indexer).
const MAX_FILE_SIZE: usize = 2 * 1024 * 1024; // 2 MiB

pub(crate) fn search_files(
    index: &SearchIndex,
    query: &str,
    limit: usize,
    indexed: bool,
    start: Instant,
) -> ToolOutput {
    // Parse query to detect glob patterns.
    let parsed = parse_search_query(query);

    let results = if parsed.globs().next().is_some() {
        // Glob search.
        index.glob_search(query, limit)
    } else {
        // Fuzzy file search.
        index.fuzzy_search(query, limit)
    };

    let items: Vec<SearchItem> = results
        .into_iter()
        .map(|r| build_search_item(r.relative_path.clone(), r.git_status, r.score))
        .collect();

    build_search_output(query, None, items.len(), items, indexed, start)
}

pub(crate) fn search_content(
    index: &SearchIndex,
    query: &str,
    limit: usize,
    indexed: bool,
    start: Instant,
) -> ToolOutput {
    let matches = index.grep(query, MAX_FILE_SIZE, DEFAULT_MAX_MATCHES, limit);

    let items: Vec<SearchItem> = matches
        .into_iter()
        .map(|m| SearchItem {
            path: m.path,
            line: Some(m.line_number),
            col: Some(m.col),
            content: Some(truncate_output(&m.line_content, 200, 1)),
            score: m.fuzzy_score.unwrap_or(0) as f64,
            git_status: None,
        })
        .collect();

    build_search_output(query, Some("content"), items.len(), items, indexed, start)
}

pub(crate) fn search_glob(
    index: &SearchIndex,
    pattern: &str,
    limit: usize,
    indexed: bool,
    start: Instant,
) -> ToolOutput {
    let results = index.glob_search(pattern, limit);

    let items: Vec<SearchItem> = results
        .into_iter()
        .map(|r| build_search_item(r.relative_path.clone(), r.git_status, r.score))
        .collect();

    build_search_output(pattern, Some("glob"), items.len(), items, indexed, start)
}

fn build_search_output(
    query: &str,
    mode: Option<&str>,
    total: usize,
    items: Vec<SearchItem>,
    indexed: bool,
    start: Instant,
) -> ToolOutput {
    let result = SearchResult {
        total,
        items,
        indexed,
    };
    let tool_args = if let Some(m) = mode {
        serde_json::json!({ "query": query, "mode": m })
    } else {
        serde_json::json!({ "query": query })
    };
    ToolOutput {
        tool_name: "search".to_owned(),
        tool_args,
        content: serde_json::to_string_pretty(&result).unwrap_or_default(),
        bytes_transferred: None,
        duration: start.elapsed(),
        status: ToolStatus::Success,
    }
}
