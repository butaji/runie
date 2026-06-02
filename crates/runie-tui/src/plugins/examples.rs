//! Example plugins demonstrating the pure message transformer pattern.
//!
//! These are reference implementations showing how to hook into the update cycle.

use crate::plugins::{Plugin, PluginRegistry};
use crate::tui::state::{AppState, Msg, Cmd};
use std::collections::HashMap;
use std::sync::RwLock;

/// LoggingPlugin - logs all messages before they are processed
pub struct LoggingPlugin {
    name: String,
}

impl LoggingPlugin {

    #[must_use]
    #[must_use]
    pub fn new() -> Self { Self { name: "logging".to_string() } }
}

impl Default for LoggingPlugin {
    fn default() -> Self { Self::new() }
}

impl Plugin for LoggingPlugin {
    fn name(&self) -> &str { &self.name }

    fn before_update(&self, msg: &Msg) -> Option<Msg> {
        eprintln!("[LoggingPlugin] before_update: {:?}", msg);
        Some(msg.clone())
    }

    fn after_update(&self, _state: &AppState, cmds: &[Cmd]) -> Vec<Msg> {
        eprintln!("[LoggingPlugin] after_update cmds: {:?}", cmds);
        vec![]
    }
}

/// MetricsPlugin - counts messages by type (uses RwLock for thread-safe interior mutability)
pub struct MetricsPlugin {
    name: String,
    counts: RwLock<HashMap<String, usize>>,
}

impl MetricsPlugin {

    #[must_use]
    #[must_use]
    pub fn new() -> Self {
        Self { name: "metrics".to_string(), counts: RwLock::new(HashMap::new()) }
    }

    pub fn get_counts(&self) -> Option<std::sync::RwLockReadGuard<HashMap<String, usize>>> {
        self.counts.read().ok()
    }
}

impl Default for MetricsPlugin {
    fn default() -> Self { Self::new() }
}

impl Plugin for MetricsPlugin {
    fn name(&self) -> &str { &self.name }

    fn before_update(&self, msg: &Msg) -> Option<Msg> {
        if let Ok(mut counts) = self.counts.write() {
            let key = format!("{:?}", msg);
            *counts.entry(key).or_insert(0) += 1;
        }
        Some(msg.clone())
    }

    fn after_update(&self, _state: &AppState, _cmds: &[Cmd]) -> Vec<Msg> {
        vec![]
    }
}

/// GitStatusPlugin - checks git status periodically and emits SetTopBar when changed
///
/// This is a REAL plugin that demonstrates the plugin system doing something useful.
/// It replaces the need for tui_run.rs to manually set top_bar.git_info.
pub struct GitStatusPlugin {
    name: String,
    /// Mutable state protected by RwLock for interior mutability in after_update
    state: RwLock<GitPluginState>,
    /// Workspace path for git commands (set via set_workspace)
    workspace: Option<std::path::PathBuf>,
}

struct GitPluginState {
    last_repo: String,
    last_branch: String,
    last_path: String,
    tick_counter: usize,
}

impl GitStatusPlugin {

    #[must_use]
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: "git_status".to_string(),
            state: RwLock::new(GitPluginState {
                last_repo: String::new(),
                last_branch: String::new(),
                last_path: String::new(),
                tick_counter: 0,
            }),
            workspace: None,
        }
    }

    /// Set the workspace path for git operations
    pub fn set_workspace(&mut self, workspace: std::path::PathBuf) {
        self.workspace = Some(workspace);
    }

    fn detect_git_info(&self) -> (String, String, String) {
        let workspace = match &self.workspace {
            Some(w) => w,
            None => return (String::new(), String::new(), String::new()),
        };

        let repo = self.detect_remote_repo(workspace);
        let branch = self.detect_branch(workspace);
        let relative_path = self.detect_relative_path(workspace);
        (repo, branch, relative_path)
    }

    fn detect_remote_repo(&self, workspace: &std::path::Path) -> String {
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

    fn detect_branch(&self, workspace: &std::path::Path) -> String {
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

    fn detect_relative_path(&self, workspace: &std::path::Path) -> String {
        std::process::Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .current_dir(workspace)
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !path.is_empty() {
                        Some(path)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .unwrap_or_default()
    }
}

impl Default for GitStatusPlugin {
    fn default() -> Self { Self::new() }
}

impl Plugin for GitStatusPlugin {
    fn name(&self) -> &str { &self.name }

    fn after_update(&self, _state: &AppState, _cmds: &[Cmd]) -> Vec<Msg> {
        let tick_counter = self.increment_tick_counter();
        if tick_counter % 60 != 0 {
            return vec![];
        }
        self.check_and_update_git_info()
    }
}

impl GitStatusPlugin {
    fn increment_tick_counter(&self) -> usize {
        if let Ok(mut s) = self.state.write() {
            let tc = s.tick_counter;
            s.tick_counter += 1;
            tc
        } else {
            0
        }
    }

    fn check_and_update_git_info(&self) -> Vec<Msg> {
        let (repo, branch, path) = self.detect_git_info();
        let changed = {
            if let Ok(s) = self.state.read() {
                repo != s.last_repo || branch != s.last_branch || path != s.last_path
            } else {
                false
            }
        };

        if changed {
            self.update_last_seen(repo.clone(), branch.clone(), path.clone());
            vec![Msg::SetTopBar {
                repo: Some(repo),
                branch: Some(branch),
                path: Some(path),
                checks_passed: None,
                checks_total: None,
                percentage: None,
                context_badges: None,
                context_pct: None,
                context_bar_pct: None,
            }]
        } else {
            vec![]
        }
    }

    fn update_last_seen(&self, repo: String, branch: String, path: String) {
        if let Ok(mut s) = self.state.write() {
            s.last_repo = repo;
            s.last_branch = branch;
            s.last_path = path;
        }
    }
}

/// Helper to register all example plugins
pub fn register_examples(registry: &mut PluginRegistry) {
    registry.register(Box::new(LoggingPlugin::new()));
    registry.register(Box::new(MetricsPlugin::new()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logging_plugin_name() {
        let p = LoggingPlugin::new();
        assert_eq!(p.name(), "logging");
    }

    #[test]
    fn test_metrics_plugin_counts() {
        let p = MetricsPlugin::new();
        assert_eq!(p.get_counts().map(|c| c.len()).unwrap_or(0), 0);
    }

    #[test]
    fn test_metrics_plugin_increments() {
        let p = MetricsPlugin::new();
        p.before_update(&Msg::Tick);
        assert_eq!(p.get_counts().and_then(|c| c.get("Tick").copied()).unwrap_or(0), 1);
    }
}
