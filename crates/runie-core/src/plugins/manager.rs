use crate::plugins::discovery::PluginDiscovery;
use crate::plugins::registry::{LoadedPlugin, PluginRegistry};
use std::path::PathBuf;

pub struct PluginManager {
    registry: PluginRegistry,
    discovery: PluginDiscovery,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            registry: PluginRegistry::new(),
            discovery: PluginDiscovery::new(),
        }
    }

    pub fn with_discovery(discovery: PluginDiscovery) -> Self {
        Self {
            registry: PluginRegistry::new(),
            discovery,
        }
    }

    pub fn discover(&mut self) -> anyhow::Result<()> {
        let discovered = self.discovery.discover();
        for dp in discovered {
            let plugin = LoadedPlugin {
                manifest: dp.manifest,
                root: dp.root,
                enabled: true,
            };
            self.registry.register(plugin);
        }
        Ok(())
    }

    pub fn install(&mut self, source: &str) -> anyhow::Result<()> {
        let path = PathBuf::from(source);
        if !path.exists() {
            anyhow::bail!("plugin source does not exist: {}", source);
        }
        let manifest_path = path.join("manifest.json");
        if !manifest_path.exists() {
            anyhow::bail!("no manifest.json found in plugin source");
        }
        let manifest = crate::plugins::manifest::PluginManifest::load(&manifest_path)?;
        let plugin = LoadedPlugin {
            manifest,
            root: path,
            enabled: true,
        };
        self.registry.register(plugin);
        Ok(())
    }

    pub fn uninstall(&mut self, name: &str) -> anyhow::Result<()> {
        if self.registry.get(name).is_none() {
            anyhow::bail!("plugin not found: {}", name);
        }
        self.registry.unregister(name);
        Ok(())
    }

    pub fn enable(&mut self, name: &str) {
        self.registry.enable(name);
    }

    pub fn disable(&mut self, name: &str) {
        self.registry.disable(name);
    }

    pub fn list_plugins(&self) -> Vec<&LoadedPlugin> {
        self.registry.list()
    }

    pub fn enabled_plugins(&self) -> Vec<&LoadedPlugin> {
        self.registry.enabled()
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}
