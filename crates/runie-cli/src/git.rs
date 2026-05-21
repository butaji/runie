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
    let repo = detect_remote_repo(workspace);
    let branch = detect_branch(workspace);
    let relative_path = detect_relative_path(workspace);
    GitInfo { repo, branch, relative_path }
}

fn detect_remote_repo(workspace: &PathBuf) -> String {
    std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(workspace)
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
                url.split('/').last()
                    .map(|s| s.trim_end_matches(".git").to_string())
                    .filter(|s| !s.is_empty())
            } else {
                None
            }
        })
        .unwrap_or_else(|| {
            workspace.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "runie".to_string())
        })
}

fn detect_branch(workspace: &PathBuf) -> String {
    std::process::Command::new("git")
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
        .unwrap_or_else(|| "main".to_string())
}

fn detect_relative_path(workspace: &PathBuf) -> String {
    std::process::Command::new("git")
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
        .unwrap_or_default()
}
