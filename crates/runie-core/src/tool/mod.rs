//! Tool definitions and shared types for Runie.
//!
//! The concrete tool implementations live in `runie-agent::tool`. This
//! module keeps the context/output/status types and pure formatting helpers
//! so that crates can depend only on core.
//!
//! ## MCP Tool Boundary (requires `mcp` feature)
//!
//! When the `mcp` feature is enabled, tools implement [`schema::ToolDef`],
//! which generates MCP-compatible schemas and handles execution.

/// MCP tool annotations. Requires the `mcp` feature.
#[cfg(feature = "mcp")]
pub mod annotations;
pub mod authorize;
pub mod cache;
pub mod circuit_breaker;
mod constraints;
mod context;
mod format;
pub mod parse;
/// MCP tool schema. Requires the `mcp` feature.
#[cfg(feature = "mcp")]
pub mod schema;
pub mod shim;
mod state;
#[cfg(test)]
mod tests;
pub mod types;

pub use authorize::{
    allow_readonly, always_ask, authorize, check_input_patterns, deny, AuthorizeResult,
    AuthorizationContext, Authorizable,
};
pub use cache::{is_cacheable_tool, CacheEntry, ToolResultCache, CACHEABLE_TOOL_NAMES};
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerRegistry, CircuitState};
pub use constraints::{
    validate as validate_constraints, validate_constraint, Constraint, ConstraintViolation,
    ValidationResult,
};
pub use context::{ToolContext, ToolOutput, ToolStatus};
pub use format::{
    compact_json_args, format_bytes, format_duration, format_tool_label, format_tool_label_parts,
    tool_error, tool_status_line, truncate_output, which_tool, which_tool_async,
};
// Path resolution utilities — expand ~ and absolutize.
use path_absolutize::Absolutize;

/// Resolve a raw path string to an absolute, normalized path relative to the
/// given working directory.
pub fn resolve_path(raw: &str, working_dir: impl AsRef<std::path::Path>) -> std::path::PathBuf {
    let working_dir = working_dir.as_ref();
    let expanded = shellexpand::tilde(raw).into_owned();
    let path = std::path::Path::new(&expanded);
    if path.is_absolute() {
        path.absolutize().unwrap_or_else(|_| path.to_path_buf())
    } else {
        working_dir
            .join(path)
            .absolutize()
            .unwrap_or_else(|_| working_dir.join(path))
    }
}
pub use parse::{
    assign_tool_call_ids, build_assistant_message, has_tool_calls, parse_tool_calls,
    parse_tool_calls_fallible, tool_parse_error_message,
};
#[cfg(feature = "mcp")]
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
