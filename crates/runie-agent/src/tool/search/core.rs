//! Search tool — unified FFF-backed search for files and content.

use crate::tool::search::fff_helpers::with_picker;
use crate::tool::search::fff_helpers::{build_error_json, build_error_json_with_instant};
use crate::tool::search::modes::{search_content, search_files, search_glob};
use crate::tool::search::types::{SearchMode, DEFAULT_LIMIT};
use crate::tool::{Tool, ToolContext, ToolOutput, ToolStatus};
use anyhow::Result;
use async_trait::async_trait;
use fff_search::{FilePicker, QueryTracker};
use runie_core::actors::FffSearchState;
use runie_core::path::resolve_path_in;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
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

#[async_trait]
impl Tool for SearchTool {
    fn name(&self) -> &str { "search" }
    fn description(&self) -> &str {
        "Unified search for files and content using FFF. Supports fuzzy file search, content search (grep), glob patterns (*.rs, **/*.ts), git-status filters (git:modified, git:untracked), and location hints (file:42:5)."
    }
    fn input_schema(&self) -> Value {
        runie_core::tool::generate_schema::<SearchInput>()
    }
    fn is_read_only(&self) -> bool { true }
    fn requires_approval(&self, _input: &Value) -> bool { false }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolOutput> {
        let start = Instant::now();
        let typed: SearchInput = serde_json::from_value(input)?;
        let mode = typed.mode.as_ref().map(|s| SearchMode::from_str(s)).unwrap_or_default();
        let path = typed.path.as_deref().unwrap_or(".");
        let limit = typed.limit.unwrap_or(DEFAULT_LIMIT);
        let full_path = resolve_path_in(path, &ctx.working_dir);
        search_impl(&typed.query, mode, &full_path, limit, start)
    }
}

pub(crate) fn search_impl(
    query: &str,
    mode: SearchMode,
    _path: &PathBuf,
    limit: usize,
    start: Instant,
) -> Result<ToolOutput> {
    let state = match FffSearchState::get() {
        Some(s) => s,
        None => {
            return build_error_json_with_instant(
                "search",
                serde_json::json!({ "query": query }),
                "FFF indexer not initialized",
                "items",
                false,
                start,
            )
        }
    };
    with_picker(
        &state,
        query.to_owned(),
        start,
        build_search_lock_error,
        build_search_not_initialized,
        |picker| {
            with_query_tracker(&state, query.to_owned(), start, |qt| {
                dispatch_search(picker, qt, query, mode, limit, start)
            })
        },
    )
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

fn with_query_tracker<F>(
    state: &FffSearchState,
    query: String,
    start: Instant,
    f: F,
) -> Result<ToolOutput>
where
    F: FnOnce(Option<&QueryTracker>) -> Result<ToolOutput>,
{
    let duration = start.elapsed();
    let guard = match state.query_tracker.read() {
        Ok(g) => g,
        Err(e) => {
            return Ok(ToolOutput {
                tool_name: "search".to_owned(),
                tool_args: serde_json::json!({ "query": query }),
                content: format!("Error acquiring query_tracker lock: {}", e),
                bytes_transferred: None,
                duration,
                status: ToolStatus::Error,
            });
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
) -> Result<ToolOutput> {
    let indexed = FffSearchState::is_indexed();
    match mode {
        SearchMode::Content => search_content(picker, query, limit, indexed, start),
        SearchMode::Glob => search_glob(picker, query, limit, indexed, start),
        SearchMode::Files | SearchMode::Mixed => {
            search_files(picker, query_tracker, query, limit, indexed, start)
        }
    }
}
