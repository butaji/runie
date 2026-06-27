//! Built-in tool implementations for the Runie agent.
//!
//! All tools implement [`runie_core::tool::ToolDef`], the single interface for
//! tool definitions, schema generation, and execution via MCP.

pub use runie_core::tool::{
    tool_error, truncate_output, which_tool, which_tool_async, ToolContext, ToolDef,
    ToolOutput, ToolStatus,
};

mod bash;
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

/// List of all built-in tool names.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_tool_names_contains_all_tools() {
        let expected = [
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
        for name in expected {
            assert!(
                BUILTIN_TOOL_NAMES.contains(&name),
                "BUILTIN_TOOL_NAMES must contain {}",
                name
            );
        }
        assert_eq!(BUILTIN_TOOL_NAMES.len(), expected.len());
    }
}
