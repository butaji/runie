//! Built-in tool implementations for the Runie agent.
//!
//! All tools implement [`runie_core::tool::ToolDef`], the single interface for
//! tool definitions, schema generation, and execution via MCP.

pub use runie_core::tool::{
    is_builtin_tool, tool_error, truncate_output, which_tool, which_tool_async, ToolContext,
    ToolDef, ToolOutput, ToolStatus, BUILTIN_TOOL_NAMES,
};

use std::time::Duration;

mod bash;
mod constants;
mod edit_file;
mod fetch_docs;
mod find;
mod find_definitions;
mod grep;
mod list_dir;
mod read_file;
mod search;
mod write_file;

pub use bash::BashTool;
pub use edit_file::EditFileTool;
pub use fetch_docs::FetchDocsTool;
pub use find::FindTool;
pub use find_definitions::FindDefinitionsTool;
pub use grep::GrepTool;
pub use list_dir::ListDirTool;
pub use read_file::ReadFileTool;
pub use search::SearchTool;
pub use write_file::WriteFileTool;

// ── Tool execution helper ──────────────────────────────────────────────────────

/// Parse JSON args into typed input and call ToolDef::execute.
///
/// This function is public so it can be called by the macro-generated dispatch
/// in `tool_registry.rs`. Both modules are in the same crate, so this is safe.
pub async fn run_tool<T: ToolDef>(
    name: &str,
    args: &serde_json::Value,
    ctx: &ToolContext,
) -> ToolOutput {
    match runie_core::tool::parse_input::<T::Input>(args) {
        Ok(i) => T::execute(i, ctx).await,
        Err(e) => ToolOutput {
            tool_name: name.to_string(),
            tool_args: args.clone(),
            content: format!("Failed to parse tool input: {e}"),
            bytes_transferred: None,
            duration: Duration::from_millis(0),
            status: ToolStatus::Error,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_tool_names_matches_core() {
        // Verify our re-export matches runie_core's canonical list
        assert_eq!(BUILTIN_TOOL_NAMES, runie_core::tool::BUILTIN_TOOL_NAMES);
    }

    #[test]
    fn builtin_tool_names_contains_all_tools() {
        // Verify the list contains all our tool implementations
        let tools = [
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
        for name in tools {
            assert!(
                BUILTIN_TOOL_NAMES.contains(&name),
                "BUILTIN_TOOL_NAMES must contain {}",
                name
            );
        }
        assert_eq!(BUILTIN_TOOL_NAMES.len(), tools.len());
    }
}
