//! ReadFile tool implementation for rig-core Tool trait.

use std::path::PathBuf;

use rig_core::completion::ToolDefinition;
use rig_core::tool::Tool;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Deserialize)]
pub struct ReadFileArgs {
    pub path: String,
    #[serde(default)]
    pub offset: Option<u64>,
    #[serde(default)]
    pub limit: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct ReadFileOutput {
    pub content: String,
    pub total_lines: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncated: Option<String>,
}

#[derive(Debug, Error)]
pub enum ReadFileError {
    #[error("file not found: {0}")]
    NotFound(String),
    #[error("read failed: {0}")]
    ReadFailed(String),
    #[error("path outside workspace: {0}")]
    PathOutsideWorkspace(String),
}

pub struct ReadFileTool {
    workspace: PathBuf,
}

impl ReadFileTool {

    #[must_use]
    pub fn new(workspace: PathBuf) -> Self {
        Self { workspace }
    }

    fn resolve_path(&self, path: &str) -> Result<PathBuf, ReadFileError> {
        let resolved = self.workspace.join(path);
        if self.contains(&resolved) {
            Ok(resolved)
        } else {
            Err(ReadFileError::PathOutsideWorkspace(path.to_string()))
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

impl Tool for ReadFileTool {
    const NAME: &'static str = "read_file";

    type Error = ReadFileError;
    type Args = ReadFileArgs;
    type Output = ReadFileOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Read the contents of a file".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path to the file"
                    },
                    "offset": {
                        "type": "integer",
                        "description": "Optional line offset to start reading from"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Optional maximum number of lines to read"
                    }
                },
                "required": ["path"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let resolved = self.resolve_path(&args.path)?;

        let content = tokio::fs::read_to_string(&resolved)
            .await
            .map_err(|e| ReadFileError::ReadFailed(format!("{}: {}", args.path, e)))?;

        let total_lines = content.lines().count();
        let mut lines: Vec<&str> = content.lines().collect();

        if let Some(offset) = args.offset {
            lines = lines.into_iter().skip(offset as usize).collect();
        }
        if let Some(limit) = args.limit {
            lines.truncate(limit as usize);
        }

        let final_content = lines.join("\n");
        let truncated = if final_content.len() > 10000 {
            Some(format!("[truncated to 10000 chars from {}]", final_content.len()))
        } else {
            None
        };

        Ok(ReadFileOutput {
            content: final_content.chars().take(10000).collect(),
            total_lines,
            truncated,
        })
    }
}
