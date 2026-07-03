//! Auto-approve writes to git-tracked files.

use std::path::Path;

use async_trait::async_trait;
use git2::Repository;

use super::{PermissionContext, PermissionPolicy, PermissionResult};

const WRITE_TOOLS: &[&str] = &["write_file", "edit_file", "append_file"];

/// Auto-approve writes to files tracked by git.
#[derive(Debug, Default, Clone, Copy)]
pub struct GitTrackedWriteApprove;

impl GitTrackedWriteApprove {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PermissionPolicy for GitTrackedWriteApprove {
    fn name(&self) -> &str {
        "git_tracked_write_approve"
    }

    fn matches(&self, ctx: &PermissionContext<'_>) -> bool {
        let is_write = WRITE_TOOLS.contains(&ctx.tool);
        let Some(path) = ctx.path else {
            return false;
        };
        is_write && is_git_tracked(path)
    }

    async fn evaluate(&self, _ctx: &PermissionContext<'_>) -> Option<PermissionResult> {
        Some(PermissionResult::Allow)
    }
}

fn is_git_tracked(path: &Path) -> bool {
    let Some(repo) = Repository::discover(path).ok() else {
        return false;
    };
    let Some(workdir) = repo.workdir() else {
        return false;
    };
    let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let canonical_workdir = workdir
        .canonicalize()
        .unwrap_or_else(|_| workdir.to_path_buf());
    let Ok(relative) = canonical_path.strip_prefix(&canonical_workdir) else {
        return false;
    };

    if head_contains(&repo, relative) {
        return true;
    }

    let Ok(index) = repo.index() else {
        return false;
    };
    index.get_path(relative, 0).is_some()
}

fn head_contains(repo: &Repository, relative: &Path) -> bool {
    let Ok(head) = repo.head() else {
        return false;
    };
    let Ok(tree) = head.peel_to_tree() else {
        return false;
    };
    tree.get_path(relative).is_ok()
}
