//! EditFile tool implementation for rig-core Tool trait.

use std::path::PathBuf;

use rig_core::completion::ToolDefinition;
use rig_core::tool::Tool;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Deserialize)]
pub struct EditFileArgs {
    pub path: String,
    pub old_string: String,
    pub new_string: String,
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Serialize)]
pub struct EditFileOutput {
    pub path: String,
    pub replacements: usize,
}

#[derive(Debug, Error)]
pub enum EditFileError {
    #[error("file not found: {0}")]
    NotFound(String),
    #[error("read failed: {0}")]
    ReadFailed(String),
    #[error("write failed: {0}")]
    WriteFailed(String),
    #[error("string not found: {0}")]
    StringNotFound(String),
    #[error("multiple matches ({0}), use force to replace all")]
    MultipleMatches(usize),
    #[error("path outside workspace: {0}")]
    PathOutsideWorkspace(String),
}

pub struct EditFileTool {
    workspace: PathBuf,
}

impl EditFileTool {

    #[must_use]
    pub fn new(workspace: PathBuf) -> Self {
        Self { workspace }
    }

    fn resolve_path(&self, path: &str) -> Result<PathBuf, EditFileError> {
        let resolved = self.workspace.join(path);
        if self.contains(&resolved) {
            Ok(resolved)
        } else {
            Err(EditFileError::PathOutsideWorkspace(path.to_string()))
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

impl Tool for EditFileTool {
    const NAME: &'static str = "edit_file";

    type Error = EditFileError;
    type Args = EditFileArgs;
    type Output = EditFileOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Edit a file by replacing old_string with new_string. Both must match exactly.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path to the file"
                    },
                    "old_string": {
                        "type": "string",
                        "description": "Exact string to replace"
                    },
                    "new_string": {
                        "type": "string",
                        "description": "Replacement string"
                    },
                    "force": {
                        "type": "boolean",
                        "description": "If true, replace all occurrences. If false (default), fail when multiple matches exist."
                    }
                },
                "required": ["path", "old_string", "new_string"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let resolved = self.resolve_path(&args.path)?;
        let content = Self::read_file(&resolved, &args.path)?;
        let occurrences = content.matches(&args.old_string).count();

        // Validate occurrence count
        Self::validate_occurrences(occurrences, &args.old_string, args.force)?;

        // Compute new content and replacement count
        let (new_content, replacement_count) = Self::compute_replace(occurrences, &content, &args.old_string, &args.new_string, args.force);

        // Write and return
        Self::write_file(&resolved, &args.path, &new_content)?;
        Ok(EditFileOutput {
            path: args.path,
            replacements: replacement_count,
        })
    }

    fn read_file(path: &PathBuf, original_path: &str) -> Result<String, EditFileError> {
        tokio::fs::read_to_string(path)
            .await
            .map_err(|e| EditFileError::ReadFailed(format!("{}: {}", original_path, e)))
    }

    fn validate_occurrences(occurrences: usize, old_string: &str, force: bool) -> Result<(), EditFileError> {
        if occurrences == 0 {
            return Err(EditFileError::StringNotFound(old_string.to_string()));
        }
        if occurrences > 1 && !force {
            return Err(EditFileError::MultipleMatches(occurrences));
        }
        Ok(())
    }

    fn compute_replace(occurrences: usize, content: &str, old_string: &str, new_string: &str, force: bool) -> (String, usize) {
        let new_content = if force {
            content.replace(old_string, new_string)
        } else {
            content.replacen(old_string, new_string, 1)
        };
        let replacement_count = if force { occurrences } else { 1 };
        (new_content, replacement_count)
    }

    fn write_file(path: &PathBuf, original_path: &str, new_content: &str) -> Result<(), EditFileError> {
        tokio::fs::write(path, new_content)
            .await
            .map_err(|e| EditFileError::WriteFailed(format!("{}: {}", original_path, e)))
    }
}
