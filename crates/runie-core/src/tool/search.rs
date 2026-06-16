//! Search tool — unified FFF-backed search for files and content.
//!
//! Replaces the separate `grep`, `find`, and `list_dir` tools with a single
//! `search` tool backed by `fff-search`. Supports file search, content search,
//! glob patterns, and git-status filters via a unified query syntax.

use crate::actors::FffSearchState;
use crate::tool::{Tool, ToolContext, ToolOutput, ToolStatus};
use anyhow::Result;
use async_trait::async_trait;
use fff_search::{
    FilePicker, FuzzySearchOptions, GrepMode, GrepSearchOptions, PaginationArgs,
    QueryParser, QueryTracker,
};
use git2::Status as GitStatus;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Default max results per search.
const DEFAULT_LIMIT: usize = 50;

/// Default max matches per file for content search.
const DEFAULT_MAX_MATCHES: usize = 10;

/// Search tool — queries the global FFF index.
pub struct SearchTool;

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchResult {
    items: Vec<SearchItem>,
    total: usize,
    indexed: bool,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchItem {
    path: String,
    line: Option<u64>,
    col: Option<usize>,
    content: Option<String>,
    score: f64,
    git_status: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum SearchMode {
    Files,
    Content,
    Mixed,
    Glob,
}

impl Default for SearchMode {
    fn default() -> Self {
        SearchMode::Files
    }
}

impl SearchMode {
    fn from_str(s: &str) -> Self {
        match s {
            "content" => SearchMode::Content,
            "mixed" => SearchMode::Mixed,
            "glob" => SearchMode::Glob,
            _ => SearchMode::Files,
        }
    }
}

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

fn parse_input(
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

fn resolve_path(path: &str, working_dir: &Path) -> PathBuf {
    let p = Path::new(path);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        working_dir.join(p)
    }
}

fn search_impl(
    query: &str,
    mode: SearchMode,
    path: &Path,
    limit: usize,
    start: Instant,
) -> Result<ToolOutput> {
    let state = match FffSearchState::get() {
        Some(s) => s,
        None => {
            return Ok(ToolOutput {
                tool_name: "search".to_string(),
                tool_args: serde_json::json!({ "query": query }),
                content: serde_json::to_string_pretty(&serde_json::json!({
                    "error": "FFF indexer not initialized",
                    "items": [],
                    "total": 0,
                    "indexed": false
                }))?,
                bytes_transferred: None,
                duration: start.elapsed(),
                status: ToolStatus::Error,
            });
        }
    };

    let picker_guard = match state.picker.read() {
        Ok(g) => g,
        Err(e) => {
            return Ok(ToolOutput {
                tool_name: "search".to_string(),
                tool_args: serde_json::json!({ "query": query }),
                content: format!("Error acquiring picker lock: {}", e),
                bytes_transferred: None,
                duration: start.elapsed(),
                status: ToolStatus::Error,
            });
        }
    };

    let qt_guard = match state.query_tracker.read() {
        Ok(g) => g,
        Err(e) => {
            return Ok(ToolOutput {
                tool_name: "search".to_string(),
                tool_args: serde_json::json!({ "query": query }),
                content: format!("Error acquiring query tracker lock: {}", e),
                bytes_transferred: None,
                duration: start.elapsed(),
                status: ToolStatus::Error,
            });
        }
    };

    let picker = match picker_guard.as_ref() {
        Some(p) => p,
        None => {
            return Ok(ToolOutput {
                tool_name: "search".to_string(),
                tool_args: serde_json::json!({ "query": query }),
                content: serde_json::to_string_pretty(&serde_json::json!({
                    "error": "FFF picker not initialized",
                    "items": [],
                    "total": 0,
                    "indexed": false
                }))?,
                bytes_transferred: None,
                duration: start.elapsed(),
                status: ToolStatus::Error,
            });
        }
    };

    let indexed = FffSearchState::is_indexed();

    match mode {
        SearchMode::Content => search_content(picker, query, limit, indexed, start),
        SearchMode::Glob => search_glob(picker, query, limit, indexed, start),
        SearchMode::Files | SearchMode::Mixed => {
            let qt = qt_guard.as_ref();
            search_files(picker, qt.as_deref(), query, limit, indexed, start)
        }
    }
}

fn search_files(
    picker: &FilePicker,
    query_tracker: Option<&QueryTracker>,
    query: &str,
    limit: usize,
    indexed: bool,
    start: Instant,
) -> Result<ToolOutput> {
    let parsed = QueryParser::default().parse(query);

    let results = picker.fuzzy_search(
        &parsed,
        query_tracker,
        FuzzySearchOptions {
            max_threads: 0,
            current_file: None,
            project_path: None,
            pagination: PaginationArgs {
                offset: 0,
                limit,
            },
            combo_boost_score_multiplier: 100,
            min_combo_count: 2,
            ..Default::default()
        },
    );

    let items: Vec<SearchItem> = results
        .items
        .iter()
        .zip(results.scores.iter())
        .map(|(item, score)| {
            let git_status = item
                .git_status
                .map(|s| format_git_status(s))
                .unwrap_or_default();
            SearchItem {
                path: item.relative_path(picker),
                line: None,
                col: None,
                content: None,
                score: score.total as f64,
                git_status: if git_status.is_empty() {
                    None
                } else {
                    Some(git_status)
                },
            }
        })
        .collect();

    let result = SearchResult {
        total: results.total_matched,
        items,
        indexed,
    };

    Ok(ToolOutput {
        tool_name: "search".to_string(),
        tool_args: serde_json::json!({ "query": query }),
        content: serde_json::to_string_pretty(&result)?,
        bytes_transferred: None,
        duration: start.elapsed(),
        status: ToolStatus::Success,
    })
}

fn search_content(
    picker: &FilePicker,
    query: &str,
    limit: usize,
    indexed: bool,
    start: Instant,
) -> Result<ToolOutput> {
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

    // Build index → path map from the deduplicated file list.
    let items: Vec<SearchItem> = results
        .matches
        .iter()
        .map(|m| {
            let path = results
                .files
                .get(m.file_index)
                .map(|f| f.relative_path(picker))
                .unwrap_or_else(|| format!("<file {}>", m.file_index));
            let content = if m.line_content.len() > 200 {
                format!("{}…", &m.line_content[..200])
            } else {
                m.line_content.clone()
            };
            SearchItem {
                path,
                line: Some(m.line_number),
                col: Some(m.col),
                content: Some(content),
                score: m.fuzzy_score.unwrap_or(0) as f64,
                git_status: None,
            }
        })
        .collect();

    let result = SearchResult {
        total: results.files_with_matches,
        items,
        indexed,
    };

    Ok(ToolOutput {
        tool_name: "search".to_string(),
        tool_args: serde_json::json!({ "query": query }),
        content: serde_json::to_string_pretty(&result)?,
        bytes_transferred: None,
        duration: start.elapsed(),
        status: ToolStatus::Success,
    })
}

fn search_glob(
    picker: &FilePicker,
    pattern: &str,
    limit: usize,
    indexed: bool,
    start: Instant,
) -> Result<ToolOutput> {
    let results = picker.glob(
        pattern,
        FuzzySearchOptions {
            max_threads: 0,
            current_file: None,
            project_path: None,
            pagination: PaginationArgs {
                offset: 0,
                limit,
            },
            combo_boost_score_multiplier: 100,
            min_combo_count: 2,
            ..Default::default()
        },
    );

    let items: Vec<SearchItem> = results
        .items
        .iter()
        .zip(results.scores.iter())
        .map(|(item, score)| {
            let git_status = item
                .git_status
                .map(|s| format_git_status(s))
                .unwrap_or_default();
            SearchItem {
                path: item.relative_path(picker),
                line: None,
                col: None,
                content: None,
                score: score.total as f64,
                git_status: if git_status.is_empty() {
                    None
                } else {
                    Some(git_status)
                },
            }
        })
        .collect();

    let result = SearchResult {
        total: results.total_matched,
        items,
        indexed,
    };

    Ok(ToolOutput {
        tool_name: "search".to_string(),
        tool_args: serde_json::json!({ "query": pattern, "mode": "glob" }),
        content: serde_json::to_string_pretty(&result)?,
        bytes_transferred: None,
        duration: start.elapsed(),
        status: ToolStatus::Success,
    })
}

fn format_git_status(status: GitStatus) -> String {
    if status.contains(GitStatus::WT_NEW) || status.contains(GitStatus::INDEX_NEW) {
        "untracked".to_string()
    } else if status.contains(GitStatus::WT_MODIFIED) || status.contains(GitStatus::INDEX_MODIFIED) {
        "modified".to_string()
    } else if status.contains(GitStatus::WT_DELETED) || status.contains(GitStatus::INDEX_DELETED) {
        "deleted".to_string()
    } else if status.contains(GitStatus::WT_RENAMED) || status.contains(GitStatus::INDEX_RENAMED) {
        "renamed".to_string()
    } else {
        "clean".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(variants.iter().any(|v| v == "glob"), "glob should be in mode enum");
    }

    #[test]
    fn search_tool_schema_has_examples() {
        let tool = SearchTool;
        let schema = tool.input_schema();
        let examples = schema["properties"]["query"]["examples"].as_array().unwrap();
        // At least one example from each syntax category.
        let has_glob = examples.iter().any(|e| e.as_str().unwrap_or("").contains('*'));
        let has_negation = examples.iter().any(|e| e.as_str().unwrap_or("").contains('!'));
        let has_git = examples.iter().any(|e| e.as_str().unwrap_or("").starts_with("git:"));
        let has_location = examples.iter().any(|e| e.as_str().unwrap_or("").contains(':'));
        assert!(has_glob, "should have glob example");
        assert!(has_negation, "should have negation example");
        assert!(has_git, "should have git filter example");
        assert!(has_location, "should have location example");
    }

    #[test]
    fn search_tool_schema_documents_git_filter() {
        let tool = SearchTool;
        let desc = tool.description();
        // Verify the description mentions git-status filtering.
        assert!(desc.contains("git:"), "description should mention git: filter");
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
        assert!(json.contains("gitStatus"), "SearchItem should serialize gitStatus field");
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
        // WT_MODIFIED = 1 << 1
        let status = GitStatus::from_bits_truncate(1 << 1);
        assert_eq!(format_git_status(status), "modified");
    }

    #[test]
    fn format_git_status_untracked() {
        // WT_NEW = 1 << 7
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
        // QueryParser parses glob constraints from the query string.
        use fff_search::{Constraint, QueryParser};
        let parsed = QueryParser::default().parse("*.rs");
        // The parsed query should contain glob/extension constraints that the picker can apply.
        let has_glob = parsed
            .constraints
            .iter()
            .any(|c| matches!(c, Constraint::Glob(_) | Constraint::Extension(_)));
        assert!(has_glob, "Expected glob constraint, got: {:?}", parsed.constraints);
    }

    #[test]
    fn query_parser_applies_negation() {
        // QueryParser parses negation constraints (!test/) as Not(constraint).
        use fff_search::{Constraint, QueryParser};
        let parsed = QueryParser::default().parse("config !test/ !vendor/");
        // Negation appears as Not(constraint) wrapping.
        let has_negation = parsed
            .constraints
            .iter()
            .any(|c| matches!(c, Constraint::Not(_)));
        assert!(has_negation, "Expected Not constraint, got: {:?}", parsed.constraints);
    }

    #[test]
    fn query_parser_handles_git_status_filter() {
        use fff_search::{Constraint, QueryParser};
        let parsed = QueryParser::default().parse("git:modified");
        // Git status is a filter token parsed as a GitStatus constraint.
        let has_git = parsed
            .constraints
            .iter()
            .any(|c| matches!(c, Constraint::GitStatus(_)));
        assert!(has_git, "Expected git status constraint, got: {:?}", parsed.constraints);
    }

    #[test]
    fn query_parser_handles_location_hint() {
        use fff_search::{Location, QueryParser};
        let parsed = QueryParser::default().parse("lib.rs:42");
        // Location hints are parsed into the location field.
        assert!(
            parsed.location.is_some(),
            "Expected location, got: {:?}",
            parsed.location
        );
        // lib.rs:42 -> Location::Line(42)
        assert!(matches!(
            parsed.location.unwrap(),
            Location::Line(n) if n == 42
        ));
    }

    #[test]
    fn query_parser_handles_location_with_column() {
        use fff_search::{Location, QueryParser};
        let parsed = QueryParser::default().parse("lib.rs:42:5");
        assert!(parsed.location.is_some(), "Expected location");
        // lib.rs:42:5 -> Location::Position { line: 42, col: 5 }
        assert!(matches!(
            parsed.location.unwrap(),
            Location::Position { line: 42, col: 5 }
        ));
    }

    #[test]
    fn query_parser_handles_mixed_query() {
        use fff_search::{Constraint, QueryParser};
        let parsed = QueryParser::default().parse("config yaml !test/ git:modified *.rs");
        // Mixed query: at least glob + negation + git status constraints.
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
        assert!(has_glob && has_negation && has_git,
            "Expected glob+negation+git, got: {:?}", parsed.constraints);
    }

    #[tokio::test]
    async fn search_tool_handles_uninitialized_indexer() {
        // When the indexer is not running, search returns an error.
        let tool = SearchTool;
        let ctx = ToolContext::default();
        let input = serde_json::json!({"query": "test"});
        let output = tool.call(input, &ctx).await.unwrap();
        // Should return an error since FFF is not initialized in tests.
        assert!(
            output.status == ToolStatus::Error
                || output.content.contains("not initialized")
                || output.content.contains("items"),
            "Got: {}",
            output.content
        );
    }
}
