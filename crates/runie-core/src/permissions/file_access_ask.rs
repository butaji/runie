//! Ask for file access outside the current working directory.

use std::path::Path;

use async_trait::async_trait;

use super::{PermissionContext, PermissionPolicy, PermissionResult};

/// Ask for any file access outside the configured cwd.
#[derive(Debug, Default, Clone, Copy)]
pub struct FileAccessAsk;

impl FileAccessAsk {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PermissionPolicy for FileAccessAsk {
    fn name(&self) -> &str {
        "file_access_ask"
    }

    fn matches(&self, ctx: &PermissionContext<'_>) -> bool {
        let Some(path) = ctx.path else {
            return false;
        };
        let Some(cwd) = ctx.cwd else {
            return false;
        };
        is_outside_cwd(path, cwd)
    }

    async fn evaluate(&self, _ctx: &PermissionContext<'_>) -> Option<PermissionResult> {
        Some(PermissionResult::Ask)
    }
}

fn is_outside_cwd(path: &Path, cwd: &Path) -> bool {
    let Ok(canonical_path) = path.canonicalize() else {
        return !path.starts_with(cwd);
    };
    let Ok(canonical_cwd) = cwd.canonicalize() else {
        return !path.starts_with(cwd);
    };
    !canonical_path.starts_with(canonical_cwd)
}
