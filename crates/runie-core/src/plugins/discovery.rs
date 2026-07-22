use crate::plugins::manifest::PluginManifest;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginScope {
    Local,   // ./runie/plugins/
    Repo,    // .runie/plugins/ in repo root
    User,    // ~/.runie/plugins/
    Config,  // paths from config
    Bundled, // Built-in plugins
}

impl PluginScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            PluginScope::Local => "local",
            PluginScope::Repo => "repo",
            PluginScope::User => "user",
            PluginScope::Config => "config",
            PluginScope::Bundled => "bundled",
        }
    }
}

pub struct DiscoveredPlugin {
    pub manifest: PluginManifest,
    pub root: PathBuf,
    pub scope: PluginScope,
}

pub struct PluginDiscovery {
    config_paths: Vec<PathBuf>,
}

impl PluginDiscovery {
    pub fn new() -> Self {
        Self {
            config_paths: Vec::new(),
        }
    }

    pub fn with_config_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.config_paths = paths;
        self
    }

    pub fn discover(&self) -> Vec<DiscoveredPlugin> {
        let mut plugins = Vec::new();
        plugins.extend(self.discover_local());
        plugins.extend(self.discover_repo());
        plugins.extend(self.discover_user());
        plugins.extend(self.discover_config());
        plugins
    }

    fn discover_local(&self) -> Vec<DiscoveredPlugin> {
        let path = PathBuf::from("./runie/plugins");
        self.scan_dir(&path, PluginScope::Local)
    }

    fn discover_repo(&self) -> Vec<DiscoveredPlugin> {
        let path = PathBuf::from(".runie/plugins");
        self.scan_dir(&path, PluginScope::Repo)
    }

    fn discover_user(&self) -> Vec<DiscoveredPlugin> {
        let Some(home) = dirs::home_dir() else {
            return Vec::new();
        };
        let path = home.join(".runie/plugins");
        self.scan_dir(&path, PluginScope::User)
    }

    fn discover_config(&self) -> Vec<DiscoveredPlugin> {
        let mut plugins = Vec::new();
        for path in &self.config_paths {
            plugins.extend(self.scan_dir(path, PluginScope::Config));
        }
        plugins
    }

    fn scan_dir(&self, dir: &PathBuf, scope: PluginScope) -> Vec<DiscoveredPlugin> {
        let mut plugins = Vec::new();
        let Ok(entries) = std::fs::read_dir(dir) else {
            return plugins;
        };
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let manifest_path = path.join("manifest.json");
            if !manifest_path.exists() {
                continue;
            }
            match PluginManifest::load(&manifest_path) {
                Ok(manifest) => {
                    plugins.push(DiscoveredPlugin {
                        manifest,
                        root: path,
                        scope,
                    });
                }
                Err(e) => {
                    tracing::warn!(?path, ?e, "failed to load plugin manifest");
                }
            }
        }
        plugins
    }
}

impl Default for PluginDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

pub fn discover_plugins() -> Vec<DiscoveredPlugin> {
    PluginDiscovery::new().discover()
}
