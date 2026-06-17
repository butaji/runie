//! Search mode implementations backed by the FFF picker.

use crate::tool::search::types::{
    build_search_item, SearchItem, SearchResult, DEFAULT_MAX_MATCHES,
};
use crate::tool::{ToolOutput, ToolStatus};
use fff_search::{
    FilePicker, FuzzySearchOptions, GrepMatch, GrepMode, GrepResult, GrepSearchOptions,
    PaginationArgs, QueryParser, QueryTracker,
};
use std::time::Instant;

pub(crate) fn search_files(
    picker: &FilePicker,
    query_tracker: Option<&QueryTracker>,
    query: &str,
    limit: usize,
    indexed: bool,
    start: Instant,
) -> anyhow::Result<ToolOutput> {
    let parsed = QueryParser::default().parse(query);
    let results = picker.fuzzy_search(
        &parsed,
        query_tracker,
        FuzzySearchOptions {
            max_threads: 0,
            current_file: None,
            project_path: None,
            pagination: PaginationArgs { offset: 0, limit },
            combo_boost_score_multiplier: 100,
            min_combo_count: 2,
        },
    );
    let items: Vec<SearchItem> = results
        .items
        .iter()
        .zip(results.scores.iter())
        .map(|(item, score)| {
            build_search_item(
                item.relative_path(picker),
                item.git_status,
                score.total as f64,
            )
        })
        .collect();
    build_search_output(query, None, results.total_matched, items, indexed, start)
}

pub(crate) fn search_content(
    picker: &FilePicker,
    query: &str,
    limit: usize,
    indexed: bool,
    start: Instant,
) -> anyhow::Result<ToolOutput> {
    let parsed = QueryParser::default().parse(query);
    let results = picker.grep(
        &parsed,
        &GrepSearchOptions {
            max_file_size: fff_search::MAX_FFFILE_SIZE,
            max_matches_per_file: DEFAULT_MAX_MATCHES,
            smart_case: true,
            file_offset: 0,
            page_limit: limit,
            mode: GrepMode::Regex,
            time_budget_ms: 5000,
            before_context: 0,
            after_context: 0,
            classify_definitions: false,
            trim_whitespace: true,
            abort_signal: None,
        },
    );
    let items: Vec<SearchItem> = results
        .matches
        .iter()
        .map(|m| map_content_match(picker, &results, m))
        .collect();
    build_search_output(query, None, results.files_with_matches, items, indexed, start)
}

pub(crate) fn search_glob(
    picker: &FilePicker,
    pattern: &str,
    limit: usize,
    indexed: bool,
    start: Instant,
) -> anyhow::Result<ToolOutput> {
    let results = picker.glob(
        pattern,
        FuzzySearchOptions {
            max_threads: 0,
            current_file: None,
            project_path: None,
            pagination: PaginationArgs { offset: 0, limit },
            combo_boost_score_multiplier: 100,
            min_combo_count: 2,
        },
    );
    let items: Vec<SearchItem> = results
        .items
        .iter()
        .zip(results.scores.iter())
        .map(|(item, score)| {
            build_search_item(
                item.relative_path(picker),
                item.git_status,
                score.total as f64,
            )
        })
        .collect();
    build_search_output(pattern, Some("glob"), results.total_matched, items, indexed, start)
}

fn map_content_match(
    picker: &FilePicker,
    results: &GrepResult<'_>,
    m: &GrepMatch,
) -> SearchItem {
    let path = results
        .files
        .get(m.file_index)
        .map(|f| f.relative_path(picker))
        .unwrap_or_else(|| format!("<file {}>", m.file_index));
    SearchItem {
        path,
        line: Some(m.line_number),
        col: Some(m.col),
        content: Some(truncate_content(&m.line_content, 200)),
        score: m.fuzzy_score.unwrap_or(0) as f64,
        git_status: None,
    }
}

fn truncate_content(content: &str, max_len: usize) -> String {
    if content.len() > max_len {
        format!("{}…", &content[..max_len])
    } else {
        content.to_string()
    }
}

fn build_search_output(
    query: &str,
    mode: Option<&str>,
    total: usize,
    items: Vec<SearchItem>,
    indexed: bool,
    start: Instant,
) -> anyhow::Result<ToolOutput> {
    let result = SearchResult { total, items, indexed };
    let tool_args = if let Some(m) = mode {
        serde_json::json!({ "query": query, "mode": m })
    } else {
        serde_json::json!({ "query": query })
    };
    Ok(ToolOutput {
        tool_name: "search".to_string(),
        tool_args,
        content: serde_json::to_string_pretty(&result)?,
        bytes_transferred: None,
        duration: start.elapsed(),
        status: ToolStatus::Success,
    })
}
