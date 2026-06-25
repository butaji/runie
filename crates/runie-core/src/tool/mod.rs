//! Tool registry and shared types for Runie.
//!
//! The concrete tool implementations live in `runie-agent::tool`. This
//! module keeps the [`Tool`] trait, [`ToolRegistry`], context/output/status
//! types, and pure formatting helpers so that crates can depend only on core.
//!
//! ## Schema-based Tool Definitions
//!
//! New tools should use the [`schema`] module with typed input structs
//! and schemars for automatic JSON schema generation.

mod context;
mod format;
mod registry;
pub mod schema;
mod state;
#[cfg(test)]
mod tests;

pub use context::{ToolContext, ToolOutput, ToolStatus};
pub use format::{
    compact_json_args, format_bytes, format_duration, format_tool_label, tool_error,
    tool_status_line, truncate_output, which_tool, which_tool_async,
};
// Re-export path utilities from canonical location
pub use crate::path::resolve_path_in as resolve_path;
pub use registry::{Tool, ToolRegistry};
pub use schema::{generate_schema, parse_input, to_mcp_tool, to_openai_function, ToolDef, ToolResult};
pub use state::{ToolCallState, ToolCallTracker};
