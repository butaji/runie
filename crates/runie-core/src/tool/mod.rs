//! Tool definitions and shared types for Runie.
//!
//! The concrete tool implementations live in `runie-agent::tool`. This
//! module keeps the [`schema::ToolDef`] trait, context/output/status
//! types, and pure formatting helpers so that crates can depend only on core.
//!
//! ## MCP Tool Boundary
//!
//! All tools implement [`schema::ToolDef`], which generates MCP-compatible schemas
//! and handles execution. There is no separate `Tool` trait or `ToolRegistry`;
//! tools are MCP tools by definition.

mod constraints;
mod context;
mod format;
pub mod parse;
pub mod schema;
pub mod shim;
mod state;
#[cfg(test)]
mod tests;
pub mod types;

pub use constraints::{
    validate as validate_constraints, validate_constraint, Constraint, ConstraintViolation,
    ValidationResult,
};
pub use context::{ToolContext, ToolOutput, ToolStatus};
pub use format::{
    compact_json_args, format_bytes, format_duration, format_tool_label, tool_error,
    tool_status_line, truncate_output, which_tool, which_tool_async,
};
// Re-export path utilities from canonical location
pub use crate::path::resolve_path_in as resolve_path;
pub use parse::{
    assign_tool_call_ids, build_assistant_message, has_tool_calls, parse_tool_calls,
    parse_tool_calls_fallible, tool_parse_error_message,
};
pub use schema::{generate_schema, parse_input, to_mcp_tool, to_openai_function, ToolDef};
pub use state::{ToolCallState, ToolCallTracker};
pub use types::{repair_partial_json, ParsedToolCall, ToolParseError};

/// Canonical list of all implemented built-in tool names.
///
/// This list is the single source of truth for:
/// - Tool registration in `runie-agent`
/// - Parsing validation in tool shims
/// - Dispatch validation in tool runners
///
/// Protocol-level names like `ask_user`, `select_model`, `done` are NOT
/// included here because they are signals, not implemented tools.
pub const BUILTIN_TOOL_NAMES: &[&str] = &[
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

/// Check if a tool name is a known built-in.
#[inline]
pub fn is_builtin_tool(name: &str) -> bool {
    BUILTIN_TOOL_NAMES.contains(&name)
}
