//! WriteFile tool implementation for rig-core Tool trait.

use std::path::PathBuf;

use rig_core::completion::ToolDefinition;
use rig_core::tool::Tool;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Deserialize)]
pub struct WriteFileArgs {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct WriteFileOutput {
    pub path: String,
    pub bytes_written: usize,
}

#[derive(Debug, Error)]
pub enum WriteFileError {
    #[error("write failed: {0}")]
    WriteFailed(String),
    #[error("path outside workspace: {0}")]
    PathOutsideWorkspace(String),
    #[error("invalid path: {0}")]
    InvalidPath(String),
}

pub struct WriteFileTool {
    workspace: PathBuf,
}

impl WriteFileTool {
    pub fn new(workspace: PathBuf) -> Self {
        Self { workspace }
    }

    fn resolve_path(&self, path: &str) -> Result<PathBuf, WriteFileError> {
        let resolved = self.workspace.join(path);
        if self.contains(&resolved) {
            Ok(resolved)
        } else {
            Err(WriteFileError::PathOutsideWorkspace(path.to_string()))
        }
    }

    fn contains(&self, path: &PathBuf) -> bool {
        let canonical_root = match self.workspace.canonicalize() {
            Ok(root) => root,
            Err(_) => return false,
        };

        let absolute_path = std::path::absolute(path).unwrap_or_else(|_| path.clone());
        let normalized = if absolute_path.is_relative() {
            canonical_root.join(absolute_path)
        } else {
            absolute_path
        };

        normalized.starts_with(&canonical_root)
    }
}

impl Tool for WriteFileTool {
    const NAME: &'static str = "write_file";

    type Error = WriteFileError;
    type Args = WriteFileArgs;
    type Output = WriteFileOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Write content to a file. Creates the file if it doesn't exist, overwrites if it does.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path to the file"
                    },
                    "content": {
                        "type": "string",
                        "description": "Content to write"
                    }
                },
                "required": ["path", "content"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let resolved = self.resolve_path(&args.path)?;

        // Validate path doesn't contain null bytes or other invalid chars
        if args.path.contains('\0') {
            return Err(WriteFileError::InvalidPath(
                "path contains null byte".to_string(),
            ));
        }

        // Create parent directories if needed
        if let Some(parent) = resolved.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| WriteFileError::WriteFailed(format!("failed to create directories: {}", e)))?;
        }

        tokio::fs::write(&resolved, &args.content)
            .await
            .map_err(|e| WriteFileError::WriteFailed(format!("{}: {}", args.path, e)))?;

        Ok(WriteFileOutput {
            path: args.path,
            bytes_written: args.content.len(),
        })
    }
}
