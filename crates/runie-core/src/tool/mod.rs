//! Tool registry and shared types for Runie.
//!
//! The concrete tool implementations have moved to `runie-engine::tool`. This
//! module keeps the [`Tool`] trait, [`ToolRegistry`], context/output/status
//! types, and pure formatting helpers so that crates can depend only on core.

mod context;
mod format;
mod registry;
pub mod runtime;
mod state;
#[cfg(test)]
mod tests;

pub use context::{ToolContext, ToolOutput, ToolStatus};
pub use format::{
    compact_json_args, format_bytes, format_duration, format_tool_label, resolve_path, tool_error,
    tool_status_line, truncate_output, which_tool,
};
pub use registry::{Tool, ToolRegistry};
pub use runtime::{
    ExecApprovalRequirement, NetworkApprovalSpec, SandboxAttempt, ToolError, ToolRuntime,
};
pub use state::{ToolCallState, ToolCallTracker};
