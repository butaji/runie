//! Git operations for agent worktrees
//! Manages git worktrees per agent and commits

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};
use std::process::Command;

/// Git worktree for an agent
#[derive(Debug, Clone)]
pub struct Worktree {
    pub path: PathBuf,
    pub branch: String,
    pub agent_id: String,
}

impl Worktree {
    /// Create a new worktree for an agent
    pub fn create(repo_path: &Path, agent_id: &str) -> anyhow::Result<Self> {
        let branch = format!("anvil/{}", agent_id);
        let worktree_path = repo_path.join(".git").join("worktrees").join(agent_id);

        // Create worktree
        let output = Command::new("git")
            .args([
                "worktree", "add",
                "-b", &branch,
                worktree_path.to_str().unwrap_or(""),
                "HEAD",
            ])
            .current_dir(repo_path)
            .output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Failed to create worktree: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        Ok(Self {
            path: worktree_path,
            branch,
            agent_id: agent_id.to_string(),
        })
    }

    /// Remove a worktree
    pub fn remove(&self, repo_path: &Path) -> anyhow::Result<()> {
        let output = Command::new("git")
            .args(["worktree", "remove", self.path.to_str().unwrap_or("")])
            .current_dir(repo_path)
            .output()?;

        if !output.status.success() {
            // Try force remove
            let output = Command::new("git")
                .args(["worktree", "remove", "--force", self.path.to_str().unwrap_or("")])
                .current_dir(repo_path)
                .output()?;

            if !output.status.success() {
                return Err(anyhow::anyhow!(
                    "Failed to remove worktree: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
        }

        // Remove branch
        let _ = Command::new("git")
            .args(["branch", "-D", &self.branch])
            .current_dir(repo_path)
            .output();

        Ok(())
    }
}

/// Git operations for anvil
#[derive(Clone)]
pub struct GitOps {
    repo_path: PathBuf,
}

impl GitOps {
    pub fn new(repo_path: PathBuf) -> Self {
        Self { repo_path }
    }

    /// Get list of changed files since last commit
    pub fn changed_files(&self) -> anyhow::Result<Vec<String>> {
        let output = Command::new("git")
            .args(["diff", "--name-only", "HEAD"])
            .current_dir(&self.repo_path)
            .output()?;

        let files = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|s| s.to_string())
            .collect();

        Ok(files)
    }

    /// Create a commit for completed task
    pub fn commit(&self, message: &str) -> anyhow::Result<String> {
        // Stage all changes
        let output = Command::new("git")
            .args(["add", "-A"])
            .current_dir(&self.repo_path)
            .output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Failed to stage changes: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        // Commit
        let output = Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(&self.repo_path)
            .output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Failed to commit: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let commit_hash = String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .unwrap_or("")
            .to_string();

        Ok(commit_hash)
    }

    /// Check if there are uncommitted changes
    pub fn has_changes(&self) -> bool {
        let output = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&self.repo_path)
            .output();

        output
            .map(|o| !o.stdout.is_empty())
            .unwrap_or(false)
    }

    /// Get current branch
    pub fn current_branch(&self) -> String {
        let output = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(&self.repo_path)
            .output();

        output
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|_| "unknown".to_string())
    }
}
