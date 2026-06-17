//! Model — Application State (mutable borrow, no cloning per event)

pub use crate::message::{now, ChatMessage, Role};
pub use crate::model::state::{
    AppState, DeliveryMode, FffFileEntry, QueuedMessage, QueuedMessageKind, ThinkingLevel,
};
pub use crate::model_catalog::{
    build_model_selector_items, filter_models, model_catalog, ModelInfo,
};
pub use crate::scoped_model::ScopedModel;

pub use crate::snapshot::{GitInfo, Snapshot};

/// Tuple representing a single model selector entry.
pub type ModelSelectorItem = (String, String, String, bool, bool);

/// Detect git repo name and current branch from the given directory.
/// Walks up the tree looking for `.git` (dir or file with `gitdir:` pointer).
pub fn detect_git_info(start: &std::path::Path) -> Option<crate::snapshot::GitInfo> {
    let mut current = Some(start);
    while let Some(dir) = current {
        let git_path = dir.join(".git");
        if git_path.is_dir() {
            return read_git_info(&git_path);
        }
        if git_path.is_file() {
            read_worktree_git_info(&git_path);
            if let Some(info) = read_worktree_git_info(&git_path) {
                return Some(info);
            }
        }
        current = dir.parent();
    }
    None
}

fn read_git_info(git_dir: &std::path::Path) -> Option<crate::snapshot::GitInfo> {
    let head_path = git_dir.join("HEAD");
    let branch = read_branch(&head_path);
    let config_path = git_dir.join("config");
    let repo_name = read_origin_repo_name(&config_path);
    Some(crate::snapshot::GitInfo {
        repo_name,
        branch,
        is_worktree: false,
        worktree_source: None,
    })
}

fn read_worktree_git_info(git_file: &std::path::Path) -> Option<crate::snapshot::GitInfo> {
    let gitdir = std::fs::read_to_string(git_file).ok().and_then(|content| {
        content
            .trim()
            .strip_prefix("gitdir:")
            .map(|s| std::path::PathBuf::from(s.trim()))
    });
    let worktree_gitdir = gitdir?;
    let head_path = worktree_gitdir.join("HEAD");
    let branch = read_branch(&head_path);
    let config_path = worktree_gitdir
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("config"));
    let repo_name = config_path.and_then(|p| read_origin_repo_name(&p));
    let worktree_source = worktree_gitdir
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .map(|p| p.to_string_lossy().to_string());
    Some(crate::snapshot::GitInfo {
        repo_name,
        branch,
        is_worktree: true,
        worktree_source,
    })
}

fn read_branch(head_path: &std::path::Path) -> Option<String> {
    std::fs::read_to_string(head_path).ok().and_then(|content| {
        content
            .trim()
            .strip_prefix("ref: refs/heads/")
            .map(|b| b.to_string())
    })
}

fn read_origin_repo_name(config_path: &std::path::Path) -> Option<String> {
    std::fs::read_to_string(config_path)
        .ok()
        .and_then(|config| {
            config
                .lines()
                .skip_while(|line| !line.contains("[remote \"origin\"]"))
                .skip(1)
                .find(|line| line.trim().starts_with("url"))
                .and_then(|url_line| {
                    let url = url_line.split('=').nth(1)?;
                    let url = url.trim();
                    url.rsplit('/')
                        .next()
                        .map(|name| name.trim_end_matches(".git").to_string())
                })
        })
}

/// Get the current working directory name.
pub fn current_dir_name() -> String {
    std::env::current_dir()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_default()
}

/// Initialize git info and cwd name once at startup.
pub fn init_git_and_cwd() -> (Option<crate::snapshot::GitInfo>, String) {
    let cwd = std::env::current_dir().ok();
    let cwd_name = cwd
        .as_ref()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let git_info = cwd.as_ref().and_then(|p| detect_git_info(p));
    (git_info, cwd_name)
}

mod cache;
mod snapshot;
mod state;
