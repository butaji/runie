//! Shared search-state accessors and error builders for search and find_definitions.
//!
//! Both tools access `FffSearchState::index` and produce `ToolOutput`
//! with not-initialized errors. The error JSON shapes are shared; tool-specific
//! formatting lives in the callers.

use crate::tool::{ToolOutput, ToolStatus};
use anyhow::Result;
use runie_core::actors::FffSearchState;
use serde_json::json;
use std::time::{Duration, Instant};

/// Callback that formats a "picker not initialized" error for a specific tool.
pub type NotInitializedBuilder = fn(String, Duration) -> ToolOutput;

/// Acquire the search index from global state.
///
/// - Returns `Err` if the indexer has not been spawned.
/// - Calls `f(index)` on success.
pub fn with_search_index<F>(
    state: &FffSearchState,
    query: String,
    start: Instant,
    not_init: NotInitializedBuilder,
    f: F,
) -> Result<ToolOutput>
where
    F: FnOnce(&runie_core::actors::fff_indexer::SearchIndex) -> Result<ToolOutput>,
{
    let duration = start.elapsed();
    if !state.indexed {
        return Ok(not_init(query, duration));
    }
    f(&state.index)
}

/// Build a JSON error output for search-related errors.
///
/// The `result_key` determines the field name: `"items"` for search or `"results"` for definitions.
pub fn build_error_json(
    tool_name: &str,
    tool_args: serde_json::Value,
    error: &str,
    result_key: &str,
    indexed: bool,
    duration: Duration,
) -> ToolOutput {
    let content = serde_json::to_string_pretty(&json!({
        "error": error,
        result_key: [],
        "total": 0,
        "indexed": indexed
    }))
    .unwrap_or_else(|_| {
        format!(
            r#"{{"error":"{}","{}":[],"total":0,"indexed":{}}}"#,
            error, result_key, indexed
        )
    });

    ToolOutput {
        tool_name: tool_name.to_owned(),
        tool_args,
        content,
        bytes_transferred: None,
        duration,
        status: ToolStatus::Error,
    }
}

/// Build a JSON error output from an Instant (computes duration internally).
pub fn build_error_json_with_instant(
    tool_name: &str,
    tool_args: serde_json::Value,
    error: &str,
    result_key: &str,
    indexed: bool,
    start: Instant,
) -> Result<ToolOutput> {
    Ok(build_error_json(
        tool_name,
        tool_args,
        error,
        result_key,
        indexed,
        start.elapsed(),
    ))
}
