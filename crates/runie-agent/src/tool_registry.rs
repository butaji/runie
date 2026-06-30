//! Centralized tool registry — single source of truth for tool names, schemas,
//! and dispatch.
//!
//! Replaces the three duplicated `build_tool_registry` functions and the
//! hard-coded match in `dispatch_tool`. Adding a new tool now requires editing
//! only this file and the tool's implementation.

use serde_json::Value;

use runie_core::tool::to_openai_function;

/// All built-in tools (read + write).
pub const ALL_TOOL_NAMES: &[&str] = &[
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

// ── Dispatch ───────────────────────────────────────────────────────────────────

/// Execute a tool by name, returning the output or an error.
/// Used by `tool_runner.rs` — replaces the hard-coded match statement.
pub fn dispatch(name: &str, args: &Value, ctx: &runie_core::tool::ToolContext) -> runie_core::tool::ToolOutput {
    // We can't use async in a sync function here, so we use
    // tokio::runtime::Handle::current().block_on as a bridge.
    // This mirrors the pattern already used elsewhere in the crate.
    let handle = match tokio::runtime::Handle::try_current() {
        Ok(h) => h,
        Err(_) => {
            return runie_core::tool::ToolOutput {
                tool_name: name.to_owned(),
                tool_args: args.clone(),
                content: "Error: no Tokio runtime present".to_owned(),
                bytes_transferred: None,
                duration: std::time::Duration::from_millis(0),
                status: runie_core::tool::ToolStatus::Error,
            };
        }
    };

    handle.block_on(async { dispatch_async(name, args, ctx).await })
}

async fn dispatch_async(
    name: &str,
    args: &Value,
    ctx: &runie_core::tool::ToolContext,
) -> runie_core::tool::ToolOutput {
    use crate::tool_runner::execute_tool_call;
    use crate::PermissionGate;
    use runie_core::permissions::{AutoAllowSink, PermissionManager};
    use std::sync::Arc;

    let tool_call = runie_core::tool::ParsedToolCall {
        name: name.to_owned(),
        args: args.clone(),
        id: None,
    };

    let gate = PermissionGate::new(PermissionManager::default(), Arc::new(AutoAllowSink));

    execute_tool_call(&tool_call, ctx, &gate).await
}
