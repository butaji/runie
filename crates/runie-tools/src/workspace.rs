use std::path::{Path, PathBuf};
use runie_core::ToolError;

#[derive(Debug, Clone)]
pub struct Workspace {
    pub root: PathBuf,
}

impl Workspace {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn resolve(&self, path: &str) -> Result<PathBuf, ToolError> {
        let resolved = self.root.join(path);
        if self.contains(&resolved) {
            Ok(resolved)
        } else {
            Err(ToolError::InvalidArguments(format!(
                "Path '{}' is outside workspace", path
            )))
        }
    }

    pub fn contains(&self, path: &Path) -> bool {
        let canonical_root = match self.root.canonicalize() {
            Ok(root) => root,
            Err(_) => return false,
        };

        let absolute_path = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
        let normalized = if absolute_path.is_relative() {
            canonical_root.join(absolute_path)
        } else {
            absolute_path
        };

        normalized.starts_with(&canonical_root)
    }
}
