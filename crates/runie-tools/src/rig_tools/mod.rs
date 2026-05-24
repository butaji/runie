//! Rig-compatible tools module.
//!
//! This module provides tools that directly implement rig-core's `Tool` trait
//! with typed arguments and outputs, rather than using the legacy runie `Tool` trait.

mod bash;
mod edit_file;
mod read_file;
mod write_file;

pub use bash::{BashTool as BashToolRig, BashArgs, BashOutput, BashError};
pub use edit_file::{EditFileTool as EditFileToolRig, EditFileArgs, EditFileOutput, EditFileError};
pub use read_file::{ReadFileTool as ReadFileToolRig, ReadFileArgs, ReadFileOutput, ReadFileError};
pub use write_file::{WriteFileTool as WriteFileToolRig, WriteFileArgs, WriteFileOutput, WriteFileError};

use std::path::PathBuf;

use rig_core::tool::{ToolDyn, ToolSet};

/// Create a rig ToolSet with all default tools pre-configured for the given workspace.
pub fn create_rig_toolset(workspace: PathBuf) -> ToolSet {
    let tools: Vec<Box<dyn ToolDyn>> = vec![
        Box::new(BashToolRig::new(workspace.clone())),
        Box::new(ReadFileToolRig::new(workspace.clone())),
        Box::new(WriteFileToolRig::new(workspace.clone())),
        Box::new(EditFileToolRig::new(workspace)),
    ];
    ToolSet::from_tools_boxed(tools)
}
