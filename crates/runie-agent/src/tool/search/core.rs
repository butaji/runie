//! Search tool — unified FFF-backed search for files and content.

use crate::tool::search::fff_helpers::with_picker;
use crate::tool::search::fff_helpers::{build_error_json, build_error_json_with_instant};
use crate::tool::search::modes::{search_content, search_files, search_glob};
use crate::tool::search::types::{SearchMode, DEFAULT_LIMIT};
use crate::tool::{ToolContext, ToolOutput, ToolStatus};
use fff_search::{FilePicker, QueryTracker};
use runie_core::actors::FffSearchState;
use runie_core::path::resolve_path_in;
use runie_core::tool::ToolDef;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;
use std::time::Instant;

/// Input parameters for search tool.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct SearchInput {
    /// Search query. Supports: fuzzy text (e.g. 'mylib'), glob (*.rs, **/*.test.ts),
    /// negation (!test/), git status (git:modified, git:untracked), and location (file:42 or file:42:5)
    pub query: String,
    /// Search mode: 'files' for fuzzy file search, 'content' for grep-style content search,
    /// 'mixed' for both, 'glob' for glob patterns (default: files)
    #[serde(default)]
    pub mode: Option<String>,
    /// Root directory to search (default: current directory)
    #[serde(default)]
    pub path: Option<String>,
    /// Maximum number of results (default: 50)
    #[serde(default)]
    pub limit: Option<usize>,
}

/// Search tool — queries the global FFF index.
pub struct SearchTool;

impl ToolDef for SearchTool {
    type Input = SearchInput;

    const NAME: &'static str = "search";
    const DESCRIPTION: &'static str = "Unified search for files and content using FFF. Supports fuzzy file search, content search (grep), glob patterns (*.rs, **/*.ts), git-status filters (git:modified, git:untracked), and location hints (file:42:5).";
    const READ_ONLY: bool = true;
    const REQUIRES_APPROVAL: bool = false;

    async fn execute(input: Self::Input, ctx: &ToolContext) -> ToolOutput {
        let start = Instant::now();
        let mode = input
            .mode
            .as_ref()
            .map(|s| SearchMode::from_str(s))
            .unwrap_or_default();
        let path = input.path.as_deref().unwrap_or(".");
        let limit = input.limit.unwrap_or(DEFAULT_LIMIT);
        let full_path = resolve_path_in(path, &ctx.working_dir);
        search_impl(&input.query, mode, &full_path, limit, start)
    }
}

fn search_error(query: &str, start: Instant, msg: String) -> ToolOutput {
    ToolOutput {
        tool_name: "search".to_owned(),
        tool_args: serde_json::json!({ "query": query }),
        content: msg,
        bytes_transferred: None,
        duration: start.elapsed(),
        status: ToolStatus::Error,
    }
}

pub(crate) fn search_impl(
    query: &str,
    mode: SearchMode,
    _path: &PathBuf,
    limit: usize,
    start: Instant,
) -> ToolOutput {
    let state = match FffSearchState::get() {
        Some(s) => s,
        None => return search_not_initialized_error(query, start),
    };
    execute_search_with_picker(&state, query, mode, limit, start)
}

fn search_not_initialized_error(query: &str, start: Instant) -> ToolOutput {
    build_error_json_with_instant(
        "search",
        serde_json::json!({ "query": query }),
        "FFF indexer not initialized",
        "items",
        false,
        start,
    )
    .unwrap_or_else(|_| search_error(query, start, "FFF indexer not initialized".to_owned()))
}

fn execute_search_with_picker(
    state: &FffSearchState,
    query: &str,
    mode: SearchMode,
    limit: usize,
    start: Instant,
) -> ToolOutput {
    with_picker(
        state,
        query.to_owned(),
        start,
        build_search_lock_error,
        build_search_not_initialized,
        |picker| {
            Ok::<ToolOutput, anyhow::Error>(with_query_tracker(
                state,
                query.to_owned(),
                start,
                |qt| dispatch_search(picker, qt, query, mode, limit, start),
            ))
        },
    )
    .unwrap_or_else(|e| search_error(query, start, format!("search error: {}", e)))
}

fn build_search_lock_error(msg: String, duration: std::time::Duration) -> ToolOutput {
    build_error_json(
        "search",
        serde_json::json!({ "query": msg }),
        &msg,
        "items",
        false,
        duration,
    )
}

fn build_search_not_initialized(query: String, duration: std::time::Duration) -> ToolOutput {
    build_error_json(
        "search",
        serde_json::json!({ "query": query }),
        "FFF picker not initialized",
        "items",
        false,
        duration,
    )
}

fn with_query_tracker<F>(state: &FffSearchState, query: String, start: Instant, f: F) -> ToolOutput
where
    F: FnOnce(Option<&QueryTracker>) -> ToolOutput,
{
    let duration = start.elapsed();
    let guard = match state.query_tracker.read() {
        Ok(g) => g,
        Err(e) => {
            return ToolOutput {
                tool_name: "search".to_owned(),
                tool_args: serde_json::json!({ "query": query }),
                content: format!("Error acquiring query_tracker lock: {}", e),
                bytes_transferred: None,
                duration,
                status: ToolStatus::Error,
            };
        }
    };
    f(guard.as_ref())
}

fn dispatch_search(
    picker: &FilePicker,
    query_tracker: Option<&QueryTracker>,
    query: &str,
    mode: SearchMode,
    limit: usize,
    start: Instant,
) -> ToolOutput {
    let indexed = FffSearchState::is_indexed();
    match mode {
        SearchMode::Content => search_content(picker, query, limit, indexed, start),
        SearchMode::Glob => search_glob(picker, query, limit, indexed, start),
        SearchMode::Files | SearchMode::Mixed => {
            search_files(picker, query_tracker, query, limit, indexed, start)
        }
    }
}
