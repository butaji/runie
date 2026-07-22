use crate::plugins::manifest::PluginManifest;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    pub root: PathBuf,
    pub enabled: bool,
}

pub struct PluginRegistry {
    plugins: HashMap<String, LoadedPlugin>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }

    pub fn register(&mut self, plugin: LoadedPlugin) {
        let name = plugin.manifest.name.clone();
        self.plugins.insert(name, plugin);
    }

    pub fn unregister(&mut self, name: &str) {
        self.plugins.remove(name);
    }

    pub fn enable(&mut self, name: &str) {
        if let Some(plugin) = self.plugins.get_mut(name) {
            plugin.enabled = true;
        }
    }

    pub fn disable(&mut self, name: &str) {
        if let Some(plugin) = self.plugins.get_mut(name) {
            plugin.enabled = false;
        }
    }

    pub fn list(&self) -> Vec<&LoadedPlugin> {
        self.plugins.values().collect()
    }

    pub fn get(&self, name: &str) -> Option<&LoadedPlugin> {
        self.plugins.get(name)
    }

    pub fn enabled(&self) -> Vec<&LoadedPlugin> {
        self.plugins.values().filter(|p| p.enabled).collect()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}
