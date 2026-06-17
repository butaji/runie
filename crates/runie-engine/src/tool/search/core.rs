//! Search tool — unified FFF-backed search for files and content.

use crate::tool::search::modes::{search_content, search_files, search_glob};
use crate::tool::search::types::{SearchMode, DEFAULT_LIMIT};
use crate::tool::{Tool, ToolContext, ToolOutput, ToolStatus};
use anyhow::Result;
use async_trait::async_trait;
use fff_search::{FilePicker, QueryTracker};
use runie_core::actors::FffSearchState;
use runie_core::tool::resolve_path;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Search tool — queries the global FFF index.
pub struct SearchTool;

#[async_trait]
impl Tool for SearchTool {
    fn name(&self) -> &str {
        "search"
    }

    fn description(&self) -> &str {
        "Unified search for files and content using FFF. \
         Supports fuzzy file search, content search (grep), glob patterns (*.rs, **/*.ts), \
         git-status filters (git:modified, git:untracked), and location hints (file:42:5)."
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query. Supports: fuzzy text (e.g. 'mylib'), glob (*.rs, **/*.test.ts), negation (!test/), git status (git:modified, git:untracked, git:staged), and location (lib.rs:42 or lib.rs:42:5)",
                    "examples": [
                        "mylib",
                        "*.rs",
                        "**/*.test.ts",
                        "config yaml !test/",
                        "git:modified",
                        "git:untracked",
                        "src/main.rs:42"
                    ]
                },
                "mode": {
                    "type": "string",
                    "enum": ["files", "content", "mixed", "glob"],
                    "description": "Search mode: 'files' for fuzzy file search, 'content' for grep-style content search, 'mixed' for both, 'glob' for glob patterns like **/*.rs (default: files)"
                },
                "path": {
                    "type": "string",
                    "description": "Root directory to search (default: current directory)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of results (default: 50)"
                }
            },
            "required": ["query"]
        })
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn requires_approval(&self, _input: &Value) -> bool {
        false
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolOutput> {
        let start = Instant::now();
        let (query, mode, path, limit) = parse_input(&input, ctx)?;
        search_impl(&query, mode, &path, limit, start)
    }
}

pub(crate) fn parse_input(
    input: &Value,
    ctx: &ToolContext,
) -> Result<(String, SearchMode, PathBuf, usize)> {
    let query = input["query"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("query is required"))?
        .to_string();
    let mode = input["mode"]
        .as_str()
        .map(SearchMode::from_str)
        .unwrap_or_default();
    let path = input["path"].as_str().unwrap_or(".");
    let limit = input["limit"].as_u64().unwrap_or(DEFAULT_LIMIT as u64) as usize;
    let full_path = resolve_path(path, &ctx.working_dir);
    Ok((query, mode, full_path, limit))
}

pub(crate) fn search_impl(
    query: &str,
    mode: SearchMode,
    _path: &Path,
    limit: usize,
    start: Instant,
) -> Result<ToolOutput> {
    let state = match FffSearchState::get() {
        Some(s) => s,
        None => return build_not_indexed_output(query, start),
    };
    with_picker(&state, query, start, |picker| {
        with_query_tracker(&state, query, start, |qt| {
            dispatch_search(picker, qt, query, mode, limit, start)
        })
    })
}

fn build_not_indexed_output(query: &str, start: Instant) -> Result<ToolOutput> {
    build_json_error_output(query, "FFF indexer not initialized", false, start)
}

fn build_picker_not_initialized_output(query: &str, start: Instant) -> Result<ToolOutput> {
    build_json_error_output(query, "FFF picker not initialized", false, start)
}

fn build_json_error_output(
    query: &str,
    error: &str,
    indexed: bool,
    start: Instant,
) -> Result<ToolOutput> {
    Ok(ToolOutput {
        tool_name: "search".to_string(),
        tool_args: serde_json::json!({ "query": query }),
        content: serde_json::to_string_pretty(&serde_json::json!({
            "error": error,
            "items": [],
            "total": 0,
            "indexed": indexed
        }))?,
        bytes_transferred: None,
        duration: start.elapsed(),
        status: ToolStatus::Error,
    })
}

fn with_picker<F>(state: &FffSearchState, query: &str, start: Instant, f: F) -> Result<ToolOutput>
where
    F: FnOnce(&FilePicker) -> Result<ToolOutput>,
{
    let guard = match state.picker.read() {
        Ok(g) => g,
        Err(e) => return Ok(build_lock_error_output(query, "picker", &e.to_string(), start)),
    };
    match guard.as_ref() {
        Some(p) => f(p),
        None => build_picker_not_initialized_output(query, start),
    }
}

fn with_query_tracker<F>(
    state: &FffSearchState,
    query: &str,
    start: Instant,
    f: F,
) -> Result<ToolOutput>
where
    F: FnOnce(Option<&QueryTracker>) -> Result<ToolOutput>,
{
    let guard = match state.query_tracker.read() {
        Ok(g) => g,
        Err(e) => {
            return Ok(build_lock_error_output(query, "query tracker", &e.to_string(), start))
        }
    };
    f(guard.as_ref())
}

fn build_lock_error_output(query: &str, resource: &str, error: &str, start: Instant) -> ToolOutput {
    ToolOutput {
        tool_name: "search".to_string(),
        tool_args: serde_json::json!({ "query": query }),
        content: format!("Error acquiring {} lock: {}", resource, error),
        bytes_transferred: None,
        duration: start.elapsed(),
        status: ToolStatus::Error,
    }
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
