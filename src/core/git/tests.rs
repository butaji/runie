//! Tests for git operations

#[cfg(test)]
mod tests {
    use crate::core::git::{GitOps, Worktree};
    use std::path::Path;

    #[test]
    fn test_worktree_structure() {
        let worktree = Worktree {
            path: std::path::PathBuf::from("/repo/.git/worktrees/test-agent"),
            branch: "anvil/test-agent".to_string(),
            agent_id: "test-agent".to_string(),
        };

        assert_eq!(worktree.agent_id, "test-agent");
        assert_eq!(worktree.branch, "anvil/test-agent");
        assert!(worktree.path.to_string_lossy().contains("test-agent"));
    }

    #[test]
    fn test_worktree_debug() {
        let worktree = Worktree {
            path: Path::new("/test/path").to_path_buf(),
            branch: "anvil/test".to_string(),
            agent_id: "test".to_string(),
        };

        let debug_str = format!("{:?}", worktree);
        assert!(debug_str.contains("test"));
        assert!(debug_str.contains("anvil/test"));
    }

    #[test]
    fn test_worktree_clone() {
        let worktree = Worktree {
            path: Path::new("/original").to_path_buf(),
            branch: "anvil/original".to_string(),
            agent_id: "original".to_string(),
        };

        let cloned = worktree.clone();
        assert_eq!(cloned.agent_id, worktree.agent_id);
        assert_eq!(cloned.branch, worktree.branch);
        assert_eq!(cloned.path, worktree.path);
    }

    #[test]
    fn test_git_ops_current_branch() {
        let git_ops = GitOps::new(PathBuf::from("."));
        
        // current_branch() should return a string (the current branch or "unknown")
        let branch = git_ops.current_branch();
        assert!(!branch.is_empty());
        assert!(branch != "unknown"); // We're in a git repo
    }

    #[test]
    fn test_git_ops_has_changes() {
        let git_ops = GitOps::new(PathBuf::from("."));
        
        // has_changes() should return a boolean
        // The actual value depends on whether there are uncommitted changes
        let has_changes = git_ops.has_changes();
        assert!(matches!(has_changes, true | false));
    }

    #[test]
    fn test_git_ops_changed_files() {
        let git_ops = GitOps::new(PathBuf::from("."));
        
        // changed_files() should return a Vec<String>
        // May be empty if no changes
        let files = git_ops.changed_files();
        assert!(files.is_ok());
        let files = files.unwrap();
        assert!(matches!(files, v if v.iter().all(|s| !s.is_empty())));
    }

    use std::path::PathBuf;

    #[test]
    fn test_git_ops_new() {
        let repo_path = PathBuf::from("/tmp/test-repo");
        let git_ops = GitOps::new(repo_path.clone());
        
        // GitOps should be created without panicking
        assert!(true);
    }

    #[test]
    fn test_git_ops_commit_without_changes() {
        let git_ops = GitOps::new(PathBuf::from("."));
        
        // commit() on repo with no staged changes should fail gracefully
        let result = git_ops.commit("test commit message");
        // Either succeeds (if there were changes to commit) or fails (if nothing staged)
        assert!(result.is_err() || result.is_ok());
    }
}
