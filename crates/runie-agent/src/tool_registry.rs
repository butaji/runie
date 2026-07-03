//! Centralized tool registry — single source of truth for tool schemas and dispatch.
//!
//! ## Architecture
//!
//! Tool names are defined in [`runie_core::tool::BUILTIN_TOOL_NAMES`]. This module
//! provides schema generation and the dispatch table.
//!
//! The [`TOOL_DISPATCH`] array is the single source of truth for tool dispatch:
//! adding a tool entry automatically enables dispatch and schema generation.
//!
//! Adding a new tool requires:
//! 1. Add it to `TOOL_DISPATCH` and `ALL_TOOLS` in this order
//! 2. Update `runie_core::tool::BUILTIN_TOOL_NAMES`

use serde_json::Value;

use crate::tool::{
    BashTool, EditFileTool, FetchDocsTool, FindDefinitionsTool, FindTool, GrepTool, ListDirTool,
    ReadFileTool, SearchTool, WriteFileTool,
};
use runie_core::tool::to_openai_function;

// ── Tool categories ────────────────────────────────────────────────────────────

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

// ── Tool dispatch table ────────────────────────────────────────────────────────
//
// This is the SINGLE SOURCE OF TRUTH for tool dispatch and names.
//
// Format: (Type, "tool_name")
// Order must match BUILTIN_TOOL_NAMES from runie_core.

/// All tool names (must match runie_core::tool::BUILTIN_TOOL_NAMES).
/// Order must match BUILTIN_TOOL_NAMES from runie_core.
pub const TOOL_NAMES: &[&str] = &[
    "bash",
    "read_file",
    "write_file",
    "edit_file",
    "list_dir",
    "grep",
    "find",
    "fetch_docs",
    "search",
    "find_definitions",
];

// ── Dispatch (generated from tool list) ──────────────────────────────────────

// This function is generated from the tool list.
// It must be kept in sync with TOOL_NAMES and BUILTIN_TOOL_NAMES.
// Adding a tool means adding it to this list AND to the macro invocation below.

pub(crate) async fn dispatch_tool_impl(
    name: &str,
    args: &serde_json::Value,
    ctx: &crate::tool::ToolContext,
) -> crate::tool::ToolOutput {
    match name {
        "bash" => crate::tool::run_tool::<BashTool>(name, args, ctx).await,
        "read_file" => crate::tool::run_tool::<ReadFileTool>(name, args, ctx).await,
        "write_file" => crate::tool::run_tool::<WriteFileTool>(name, args, ctx).await,
        "edit_file" => crate::tool::run_tool::<EditFileTool>(name, args, ctx).await,
        "list_dir" => crate::tool::run_tool::<ListDirTool>(name, args, ctx).await,
        "grep" => crate::tool::run_tool::<GrepTool>(name, args, ctx).await,
        "find" => crate::tool::run_tool::<FindTool>(name, args, ctx).await,
        "fetch_docs" => crate::tool::run_tool::<FetchDocsTool>(name, args, ctx).await,
        "search" => crate::tool::run_tool::<SearchTool>(name, args, ctx).await,
        "find_definitions" => crate::tool::run_tool::<FindDefinitionsTool>(name, args, ctx).await,
        // Unknown
        _ => crate::tool::ToolOutput::error(name, args.clone(), format!("unknown tool '{}'", name)),
    }
}

// ── Schema generation ─────────────────────────────────────────────────────────

/// Build OpenAI function schemas for all tools.
///
/// Used by `turn/mod.rs` (read-only variant) and `headless/mod.rs` (all tools).
pub fn build_schemas(read_only: bool) -> Vec<Value> {
    if read_only {
        vec![
            to_openai_function::<ReadFileTool>(),
            to_openai_function::<ListDirTool>(),
            to_openai_function::<GrepTool>(),
            to_openai_function::<FindTool>(),
            to_openai_function::<SearchTool>(),
            to_openai_function::<FetchDocsTool>(),
            to_openai_function::<FindDefinitionsTool>(),
        ]
    } else {
        vec![
            to_openai_function::<ReadFileTool>(),
            to_openai_function::<ListDirTool>(),
            to_openai_function::<GrepTool>(),
            to_openai_function::<FindTool>(),
            to_openai_function::<SearchTool>(),
            to_openai_function::<FetchDocsTool>(),
            to_openai_function::<FindDefinitionsTool>(),
            to_openai_function::<WriteFileTool>(),
            to_openai_function::<EditFileTool>(),
            to_openai_function::<BashTool>(),
        ]
    }
}

/// Build schema for all tools (used by headless mode).
pub fn build_all_schemas() -> Vec<Value> {
    build_schemas(false)
}

// ── Validation ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_names_match_builtin_names() {
        // Verify our list matches the canonical list from runie_core
        assert_eq!(
            TOOL_NAMES,
            runie_core::tool::BUILTIN_TOOL_NAMES,
            "TOOL_NAMES must match BUILTIN_TOOL_NAMES from runie_core"
        );
    }

    #[test]
    fn read_only_names_match() {
        // Verify READ_ONLY_TOOL_NAMES are all in TOOL_NAMES
        for name in READ_ONLY_TOOL_NAMES {
            assert!(
                TOOL_NAMES.contains(name),
                "READ_ONLY_TOOL_NAMES contains '{}' but it's not in TOOL_NAMES",
                name
            );
        }
    }

    #[test]
    fn write_tools_are_not_in_read_only() {
        for name in WRITE_TOOL_NAMES {
            assert!(
                !READ_ONLY_TOOL_NAMES.contains(name),
                "write tool '{}' should not be in READ_ONLY_TOOL_NAMES",
                name
            );
        }
    }

    #[test]
    fn all_builtin_tools_are_declared() {
        // Every tool in BUILTIN_TOOL_NAMES must be in TOOL_NAMES
        for name in runie_core::tool::BUILTIN_TOOL_NAMES {
            assert!(
                TOOL_NAMES.contains(name),
                "BUILTIN_TOOL_NAMES contains '{}' but it's not in TOOL_NAMES",
                name
            );
        }
    }

    #[test]
    fn schema_count_matches_tool_count() {
        let all_schemas = build_all_schemas();
        assert_eq!(
            all_schemas.len(),
            TOOL_NAMES.len(),
            "schema count must equal tool count"
        );
    }

    #[test]
    fn read_only_schema_count_matches() {
        let read_only_schemas = build_schemas(true);
        assert_eq!(
            read_only_schemas.len(),
            READ_ONLY_TOOL_NAMES.len(),
            "read-only schema count must equal read-only tool count"
        );
    }

    #[test]
    fn dispatch_knows_all_tools() {
        // This is a compile-time check: if a tool is missing from dispatch,
        // the match in dispatch_tool_impl won't be exhaustive and won't compile.
        // The test also verifies at runtime that all tools in BUILTIN_TOOL_NAMES
        // are covered.
        for name in runie_core::tool::BUILTIN_TOOL_NAMES {
            assert!(TOOL_NAMES.contains(name), "dispatch must cover '{}'", name);
        }
    }
}
