//! Git status helpers for FFF indexer.
//!
//! Provides functions for matching and formatting git2 status flags.

use git2::Status as GitStatus;

/// Returns true if the git status matches the given filter string.
pub(super) fn git_status_matches(status: GitStatus, filter: &str) -> bool {
    match filter {
        "modified" => {
            status.contains(GitStatus::WT_MODIFIED) || status.contains(GitStatus::INDEX_MODIFIED)
        }
        "untracked" => status.contains(GitStatus::WT_NEW) || status.contains(GitStatus::INDEX_NEW),
        "deleted" => {
            status.contains(GitStatus::WT_DELETED) || status.contains(GitStatus::INDEX_DELETED)
        }
        "renamed" => {
            status.contains(GitStatus::WT_RENAMED) || status.contains(GitStatus::INDEX_RENAMED)
        }
        "staged" => {
            status.contains(GitStatus::INDEX_MODIFIED)
                || status.contains(GitStatus::INDEX_NEW)
                || status.contains(GitStatus::INDEX_DELETED)
                || status.contains(GitStatus::INDEX_RENAMED)
        }
        "clean" => status.is_empty(),
        _ => false,
    }
}

/// Format a git2 Status as a human-readable label string.
///
/// Priority order mirrors `git_status_matches`: staged flags take precedence,
/// then unstaged, then renamed/deleted, then "untracked". Returns `"clean"`
/// when no tracked-change flags are set.
pub fn format_git_status(status: git2::Status) -> &'static str {
    // Staged flags — most important for display.
    if status.contains(GitStatus::INDEX_MODIFIED)
        || status.contains(GitStatus::INDEX_NEW)
        || status.contains(GitStatus::INDEX_DELETED)
        || status.contains(GitStatus::INDEX_RENAMED)
    {
        return if status.contains(GitStatus::INDEX_DELETED) {
            "deleted"
        } else if status.contains(GitStatus::INDEX_RENAMED) {
            "renamed"
        } else if status.contains(GitStatus::INDEX_NEW) {
            "untracked"
        } else {
            "modified"
        };
    }
    // Unstaged flags.
    if status.contains(GitStatus::WT_MODIFIED)
        || status.contains(GitStatus::WT_NEW)
        || status.contains(GitStatus::WT_DELETED)
        || status.contains(GitStatus::WT_RENAMED)
    {
        return if status.contains(GitStatus::WT_DELETED) {
            "deleted"
        } else if status.contains(GitStatus::WT_RENAMED) {
            "renamed"
        } else if status.contains(GitStatus::WT_NEW) {
            "untracked"
        } else {
            "modified"
        };
    }
    "clean"
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::Status;

    #[test]
    fn format_git_status_index_modified() {
        assert_eq!(format_git_status(Status::INDEX_MODIFIED), "modified");
    }

    #[test]
    fn format_git_status_index_new() {
        assert_eq!(format_git_status(Status::INDEX_NEW), "untracked");
    }

    #[test]
    fn format_git_status_index_deleted() {
        assert_eq!(format_git_status(Status::INDEX_DELETED), "deleted");
    }

    #[test]
    fn format_git_status_index_renamed() {
        assert_eq!(format_git_status(Status::INDEX_RENAMED), "renamed");
    }

    #[test]
    fn format_git_status_wt_modified() {
        assert_eq!(format_git_status(Status::WT_MODIFIED), "modified");
    }

    #[test]
    fn format_git_status_wt_new() {
        assert_eq!(format_git_status(Status::WT_NEW), "untracked");
    }

    #[test]
    fn format_git_status_empty() {
        assert_eq!(format_git_status(Status::empty()), "clean");
    }

    #[test]
    fn git_status_matches_modified() {
        assert!(git_status_matches(Status::INDEX_MODIFIED, "modified"));
        assert!(git_status_matches(Status::WT_MODIFIED, "modified"));
        assert!(!git_status_matches(Status::empty(), "modified"));
    }

    #[test]
    fn git_status_matches_untracked() {
        assert!(git_status_matches(Status::INDEX_NEW, "untracked"));
        assert!(git_status_matches(Status::WT_NEW, "untracked"));
    }

    #[test]
    fn git_status_matches_deleted() {
        assert!(git_status_matches(Status::INDEX_DELETED, "deleted"));
        assert!(git_status_matches(Status::WT_DELETED, "deleted"));
    }

    #[test]
    fn git_status_matches_staged() {
        assert!(git_status_matches(Status::INDEX_MODIFIED, "staged"));
        assert!(git_status_matches(Status::INDEX_NEW, "staged"));
        assert!(git_status_matches(Status::INDEX_DELETED, "staged"));
        assert!(git_status_matches(Status::INDEX_RENAMED, "staged"));
        assert!(!git_status_matches(Status::WT_MODIFIED, "staged"));
    }
}
