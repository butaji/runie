pub mod bash;
pub mod edit_file;
pub mod read_file;
pub mod registry;
pub mod search;
pub mod workspace;
pub mod write_file;

pub use bash::BashTool;
pub use edit_file::EditFileTool;
pub use read_file::ReadFileTool;
pub use registry::ToolRegistry;
pub use search::SearchTool;
pub use workspace::Workspace;
pub use write_file::WriteFileTool;

#[cfg(test)]
mod tests;

/// Create a registry pre-populated with the default toolkit tools.
pub fn create_default_toolkit(workspace: Workspace) -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(ReadFileTool::new(workspace.clone())));
    registry.register(Box::new(WriteFileTool::new(workspace.clone())));
    registry.register(Box::new(EditFileTool::new(workspace.clone())));
    registry.register(Box::new(BashTool::new(workspace.clone())));
    registry.register(Box::new(SearchTool::new(workspace)));
    registry
}
