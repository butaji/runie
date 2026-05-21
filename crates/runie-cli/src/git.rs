use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct GitInfo {
    pub repo: String,
    pub branch: String,
    pub relative_path: String,
}

impl Default for GitInfo {
    fn default() -> Self {
        Self {
            repo: "runie".to_string(),
            branch: "main".to_string(),
            relative_path: String::new(),
        }
    }
}

pub fn detect_git_info(workspace: &PathBuf) -> GitInfo {
    // Try to get repo name from git remote
    let repo = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(workspace)
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
                // Extract repo name from URL
                // e.g., git@github.com:user/repo.git → repo
                // e.g., https://github.com/user/repo.git → repo
                url.split('/').last()
                    .map(|s| s.trim_end_matches(".git").to_string())
                    .filter(|s| !s.is_empty())
            } else {
                None
            }
        })
        .unwrap_or_else(|| {
            // Fallback: use directory name
            workspace.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "runie".to_string())
        });

    // Try to get current branch
    let branch = std::process::Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(workspace)
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !branch.is_empty() { Some(branch) } else { None }
            } else {
                None
            }
        })
        .unwrap_or_else(|| "main".to_string());

    // Try to get path relative to git root
    let relative_path = std::process::Command::new("git")
        .args(["rev-parse", "--show-prefix"])
        .current_dir(workspace)
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    Some(path.trim_end_matches('/').to_string())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .unwrap_or_default();

    GitInfo { repo, branch, relative_path }
}
