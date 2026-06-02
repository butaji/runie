//! Plugin loader - loads plugins from filesystem
//!
//! Scans directories for plugin.toml manifests and registers plugins.

use crate::{Plugin, PluginAction, PluginCommand, PluginEvent, PluginMetadata, ExtensionType, CommandHandler};
use crate::error::ExtError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Plugin manifest parsed from plugin.toml
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PluginManifest {
    pub plugin: PluginSection,
    #[serde(default)]
    pub commands: HashMap<String, CommandSpec>,
    #[serde(default)]
    pub hooks: HashMap<String, HookSpec>,
}

/// Plugin metadata section
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PluginSection {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
}

/// Command specification from TOML
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CommandSpec {
    pub description: String,
    pub handler: String,
    #[serde(default)]
    pub args: Vec<CommandArgSpec>,
}

/// Command argument specification
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CommandArgSpec {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub required: bool,
    pub default: Option<String>,
}

/// Hook specification from TOML
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HookSpec {
    pub handler: String,
}

/// Plugin loader - scans directories for plugins
pub struct PluginLoader {
    manifests: Vec<PluginManifest>,
}

impl PluginLoader {
    pub fn new() -> Self {
        Self { manifests: Vec::new() }
    }

    /// Load all plugins from a directory
    ///
    /// Scans for `plugin.toml` files in subdirectories of the given path.
    /// Each subdirectory containing a `plugin.toml` is treated as a plugin.
    pub fn load_from_dir(&mut self, dir: &Path) -> Result<(), ExtError> {
        if !dir.exists() {
            tracing::debug!("Plugin directory does not exist: {:?}", dir);
            return Ok(());
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry.map_err(|e| ExtError::IoError(e.to_string()))?;
            let path = entry.path();

            if path.is_dir() {
                let manifest_path = path.join("plugin.toml");
                if manifest_path.exists() {
                    match self.load_manifest(&manifest_path) {
                        Ok(manifest) => {
                            tracing::info!("Loaded plugin manifest: {}", manifest.plugin.name);
                            self.manifests.push(manifest);
                        }
                        Err(e) => {
                            tracing::warn!("Failed to load plugin manifest {:?}: {}", manifest_path, e);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Load a single plugin manifest
    pub fn load_manifest(&mut self, path: &Path) -> Result<PluginManifest, ExtError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ExtError::IoError(e.to_string()))?;

        let manifest: PluginManifest = toml::from_str(&content)
            .map_err(|e| ExtError::ParseError(e.to_string()))?;

        Ok(manifest)
    }

    /// Get all loaded manifests
    pub fn manifests(&self) -> &[PluginManifest] {
        &self.manifests
    }
}

impl Default for PluginLoader {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Built-in plugins
// ─────────────────────────────────────────────────────────────────────────────

/// GitPlugin - tracks git status and provides git-related functionality
pub struct GitPlugin {
    metadata: PluginMetadata,
    workspace: Option<PathBuf>,
    last_status_check: std::time::Instant,
    status: GitStatus,
}

#[derive(Debug, Clone, Default)]
struct GitStatus {
    branch: String,
    dirty: bool,
    staged: bool,
    ahead: usize,
    behind: usize,
    conflicted: usize,
}

impl GitPlugin {
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata {
                id: "git".to_string(),
                name: "git".to_string(),
                version: "1.0.0".to_string(),
                description: Some("Git integration for tracking file changes and repository status".to_string()),
                author: Some("runie".to_string()),
                extension_type: ExtensionType::Plugin,
                dependencies: vec![],
            },
            workspace: None,
            last_status_check: std::time::Instant::now(),
            status: GitStatus::default(),
        }
    }

    pub fn set_workspace(&mut self, workspace: PathBuf) {
        self.workspace = Some(workspace);
    }

    fn check_git_status(&mut self) {
        let workspace = match &self.workspace {
            Some(w) => w,
            None => return,
        };

        // Throttle checks to once per second
        if self.last_status_check.elapsed() < std::time::Duration::from_secs(1) {
            return;
        }
        self.last_status_check = std::time::Instant::now();

        // Get branch
        self.status.branch = std::process::Command::new("git")
            .args(["branch", "--show-current"])
            .current_dir(workspace)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default();

        // Get status
        let status_output = std::process::Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(workspace)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();

        let mut dirty = false;
        let mut staged = false;
        let mut conflicted = 0;

        for line in status_output.lines() {
            if line.is_empty() {
                continue;
            }
            let idx = line.chars().next().unwrap_or(' ');
            let second = line.chars().nth(1).unwrap_or(' ');

            // First char: unstaged changes
            if idx == '?' || idx == 'M' || idx == 'D' {
                dirty = true;
            }
            // Second char: staged changes
            if second == 'M' || second == 'D' || second == 'A' {
                staged = true;
            }
            // Conflicted files have both chars set to special values
            if idx == 'U' || second == 'U' {
                conflicted += 1;
            }
        }

        self.status.dirty = dirty;
        self.status.staged = staged;
        self.status.conflicted = conflicted;
    }
}

impl Default for GitPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for GitPlugin {
    fn name(&self) -> &str {
        &self.metadata.name
    }

    fn version(&self) -> &str {
        &self.metadata.version
    }

    fn description(&self) -> Option<&str> {
        self.metadata.description.as_deref()
    }

    fn commands(&self) -> Vec<PluginCommand> {
        vec![
            PluginCommand::new("git status", "Show git status", CommandHandler::Sync(Arc::new(|_args: &[String]| {
                vec![]
            }))),
        ]
    }

    fn on_event(&self, event: PluginEvent) -> Vec<PluginAction> {
        match event {
            PluginEvent::FileEdited { path, .. } => {
                // File was edited - could trigger status refresh
                tracing::debug!("Git: file edited {}", path);
                vec![]
            }
            _ => vec![],
        }
    }

    fn on_load(&self) -> Result<(), String> {
        tracing::info!("GitPlugin loaded");
        Ok(())
    }
}

/// TimerPlugin - tracks time spent in sessions
pub struct TimerPlugin {
    metadata: PluginMetadata,
    session_start: Option<std::time::Instant>,
    total_duration: std::time::Duration,
    tool_times: HashMap<String, std::time::Duration>,
    current_tool: Option<(String, std::time::Instant)>,
}

impl TimerPlugin {
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata {
                id: "timer".to_string(),
                name: "timer".to_string(),
                version: "1.0.0".to_string(),
                description: Some("Tracks time spent in sessions and per-tool".to_string()),
                author: Some("runie".to_string()),
                extension_type: ExtensionType::Plugin,
                dependencies: vec![],
            },
            session_start: None,
            total_duration: std::time::Duration::ZERO,
            tool_times: HashMap::new(),
            current_tool: None,
        }
    }

    fn start_session(&mut self) {
        if self.session_start.is_none() {
            self.session_start = Some(std::time::Instant::now());
            tracing::debug!("Timer: session started");
        }
    }

    fn end_session(&mut self) {
        if let Some(start) = self.session_start.take() {
            let elapsed = start.elapsed();
            self.total_duration += elapsed;
            tracing::debug!("Timer: session ended, total duration: {:?}", self.total_duration);
        }
    }

    fn start_tool(&mut self, tool_name: &str) {
        // End any current tool
        self.end_tool();
        self.current_tool = Some((tool_name.to_string(), std::time::Instant::now()));
    }

    fn end_tool(&mut self) {
        if let Some((name, start)) = self.current_tool.take() {
            let elapsed = start.elapsed();
            *self.tool_times.entry(name).or_insert(std::time::Duration::ZERO) += elapsed;
        }
    }

    pub fn total_duration(&self) -> std::time::Duration {
        self.total_duration
    }

    pub fn tool_times(&self) -> &HashMap<String, std::time::Duration> {
        &self.tool_times
    }
}

impl Default for TimerPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for TimerPlugin {
    fn name(&self) -> &str {
        &self.metadata.name
    }

    fn version(&self) -> &str {
        &self.metadata.version
    }

    fn description(&self) -> Option<&str> {
        self.metadata.description.as_deref()
    }

    fn commands(&self) -> Vec<PluginCommand> {
        vec![
            PluginCommand::new("timer stats", "Show time statistics", CommandHandler::Sync(Arc::new(|_args: &[String]| {
                vec![]
            }))),
        ]
    }

    fn on_event(&self, event: PluginEvent) -> Vec<PluginAction> {
        match event {
            PluginEvent::SessionStarted { .. } => {
                vec![PluginAction::ShowToast {
                    message: "Timer: session started".to_string(),
                    duration_ms: 2000,
                }]
            }
            PluginEvent::SessionEnded { .. } => {
                vec![PluginAction::ShowToast {
                    message: "Timer: session ended".to_string(),
                    duration_ms: 2000,
                }]
            }
            PluginEvent::ToolCalled { ref tool_name, .. } => {
                tracing::debug!("Timer: tool called {}", tool_name);
                vec![]
            }
            PluginEvent::ToolResult { ref tool_name, .. } => {
                tracing::debug!("Timer: tool {} completed", tool_name);
                vec![]
            }
            _ => vec![],
        }
    }

    fn on_load(&self) -> Result<(), String> {
        tracing::info!("TimerPlugin loaded");
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Static plugin registration helper
// ─────────────────────────────────────────────────────────────────────────────

/// Registry for static plugins (compile-time registered)
pub struct StaticPluginRegistry {
    plugins: Vec<Arc<dyn Plugin>>,
}

impl StaticPluginRegistry {
    pub fn new() -> Self {
        Self { plugins: Vec::new() }
    }

    pub fn register(&mut self, plugin: Arc<dyn Plugin>) {
        self.plugins.push(plugin);
    }

    pub fn plugins(&self) -> &[Arc<dyn Plugin>] {
        &self.plugins
    }
}

impl Default for StaticPluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}
