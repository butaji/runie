//! Centralized tool registry — single source of truth for tool schemas.
//!
//! Tool names are defined in `runie_core::tool::BUILTIN_TOOL_NAMES`.
//! This module provides schema generation and tool categorization.

use serde_json::Value;

use runie_core::tool::to_openai_function;

/// Read-only tools (no write side effects — allowed in read-only mode).
pub const READ_ONLY_TOOL_NAMES: &[&str] = &[
    "read_file",
    "list_dir",
    "grep",
    "find",
    "fetch_docs",
    "search",
    "find_definitions",
];

/// Write tools (require trust / read-write mode).
pub const WRITE_TOOL_NAMES: &[&str] = &["bash", "write_file", "edit_file"];

// ── Schema generation ──────────────────────────────────────────────────────────

/// Build OpenAI function schemas for all tools.
///
/// Used by `turn/mod.rs` (read-only variant) and `headless/mod.rs` (all tools).
pub fn build_schemas(read_only: bool) -> Vec<Value> {
    use crate::tool::{
        BashTool, EditFileTool, FetchDocsTool, FindDefinitionsTool, FindTool, GrepTool,
        ListDirTool, ReadFileTool, SearchTool, WriteFileTool,
    };

    let mut schemas = vec![
        to_openai_function::<ReadFileTool>(),
        to_openai_function::<ListDirTool>(),
        to_openai_function::<GrepTool>(),
        to_openai_function::<FindTool>(),
        to_openai_function::<SearchTool>(),
        to_openai_function::<FetchDocsTool>(),
        to_openai_function::<FindDefinitionsTool>(),
    ];

    if !read_only {
        schemas.push(to_openai_function::<WriteFileTool>());
        schemas.push(to_openai_function::<EditFileTool>());
        schemas.push(to_openai_function::<BashTool>());
    }

    schemas
}

/// Build schema for all tools (used by headless mode).
pub fn build_all_schemas() -> Vec<Value> {
    build_schemas(false)
}
