use std::path::{Path, PathBuf};
use tidy_core::ToolError;

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
        path.canonicalize()
            .and_then(|canonical| {
                self.root.canonicalize().map(|root| canonical.starts_with(&root))
            })
            .unwrap_or(false)
    }
}
