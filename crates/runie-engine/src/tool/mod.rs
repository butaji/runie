//! Built-in tool implementations for the Runie engine.
//!
//! Tool trait and shared types live in `runie_core::tool`; this module provides
//! the concrete implementations and the canonical [`builtin_registry`].

use std::sync::Arc;

pub use runie_core::tool::{which_tool, which_tool_async, Tool, ToolContext, ToolOutput, ToolRegistry, ToolStatus};

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

/// Create a registry with all built-in tools registered.
pub fn builtin_registry() -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    registry.register(Arc::new(BashTool));
    registry.register(Arc::new(ReadFileTool));
    registry.register(Arc::new(WriteFileTool));
    registry.register(Arc::new(EditFileTool));
    registry.register(Arc::new(ListDirTool));
    registry.register(Arc::new(GrepTool));
    registry.register(Arc::new(FindTool));
    registry.register(Arc::new(FetchDocsTool));
    registry.register(Arc::new(SearchTool));
    registry.register(Arc::new(FindDefinitionsTool));
    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_registry_unique() {
        let registry = builtin_registry();
        let names: std::collections::HashSet<String> = registry
            .list()
            .iter()
            .map(|t| t.name().to_string())
            .collect();
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
                names.contains(name),
                "builtin_registry must contain {}",
                name
            );
        }
        assert_eq!(names.len(), expected.len());
    }

    #[tokio::test]
    async fn registry_filters_builtin_tools() {
        let registry = builtin_registry();
        let filtered = registry.filtered(&["read_file".to_string(), "grep".to_string()]);
        assert!(filtered.get("read_file").is_some());
        assert!(filtered.get("grep").is_some());
        assert!(filtered.get("bash").is_none());
    }
}
