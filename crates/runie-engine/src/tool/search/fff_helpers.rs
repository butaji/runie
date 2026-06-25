//! Shared FFF-state accessors and error builders for search and find_definitions.
//!
//! Both tools wait on `FffSearchState::picker.read()` and produce `ToolOutput`
//! with lock-poisoning and not-initialized errors. The lock-guard pattern and
//! error JSON shapes are shared; tool-specific formatting lives in the callers.

use anyhow::Result;
use crate::tool::{ToolOutput, ToolStatus};
use fff_search::FilePicker;
use runie_core::actors::FffSearchState;
use serde_json::json;
use std::time::{Duration, Instant};

/// Callback that formats a lock-poisoning error for a specific tool.
pub type LockErrorBuilder = fn(String, Duration) -> ToolOutput;

/// Callback that formats a "picker not initialized" error for a specific tool.
pub type NotInitializedBuilder = fn(String, Duration) -> ToolOutput;

/// Acquire the FFF picker from global state with proper lock handling.
///
/// - Returns `Err` (via `lock_err`) if the read lock is poisoned.
/// - Returns `Err` (via `not_init`) if the picker is `None`.
/// - Calls `f(picker)` on success.
pub fn with_picker<F>(
    state: &FffSearchState,
    query: String,
    start: Instant,
    lock_err: LockErrorBuilder,
    not_init: NotInitializedBuilder,
    f: F,
) -> Result<ToolOutput>
where
    F: FnOnce(&FilePicker) -> Result<ToolOutput>,
{
    let duration = start.elapsed();
    let guard = match state.picker.read() {
        Ok(g) => g,
        Err(e) => return Ok(lock_err(format!("Error acquiring picker lock: {}", e), duration)),
    };
    match guard.as_ref() {
        Some(p) => f(p),
        None => Ok(not_init(query, duration)),
    }
}

/// Build a JSON error output for FFF-related errors.
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
    .unwrap_or_else(|_| format!(r#"{{"error":"{}","{}":[],"total":0,"indexed":{}}}"#, error, result_key, indexed));

    ToolOutput {
        tool_name: tool_name.to_string(),
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
    Ok(build_error_json(tool_name, tool_args, error, result_key, indexed, start.elapsed()))
}
