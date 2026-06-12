//! Extension registry - runtime plugin management
//!
//! The registry manages lifecycle of all extensions:
//! - Registration (static via macro, dynamic via config)
//! - Loading (from dynamic libraries or subprocesses)
//! - Event dispatch (to appropriate hooks)
//! - Command routing (to plugins)

use crate::{Plugin, PluginEvent, PluginAction, PluginMetadata, ExtensionType, PluginCommand};
use crate::hooks::HookRegistry;
use crate::skills::SkillRegistry;
use crate::mcp::McpRegistry;
use crate::error::ExtError;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;

/// Unique identifier for a loaded extension
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExtensionId(pub String);

impl std::fmt::Display for ExtensionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Loaded extension wrapper
pub struct LoadedExtension {
    pub id: ExtensionId,
    pub metadata: PluginMetadata,
    instance: Arc<dyn Plugin>,
}

impl LoadedExtension {
    pub fn new(plugin: Arc<dyn Plugin>) -> Self {
        let metadata = plugin.metadata();
        Self {
            id: ExtensionId(metadata.id.clone()),
            metadata,
            instance: plugin,
        }
    }

    pub fn plugin(&self) -> &Arc<dyn Plugin> {
        &self.instance
    }
}

/// Global extension registry
pub struct ExtensionRegistry {
    plugins: RwLock<HashMap<ExtensionId, LoadedExtension>>,
    hooks: HookRegistry,
    skills: SkillRegistry,
    mcp: McpRegistry,
    command_sender: mpsc::UnboundedSender<PluginCommand>,
    event_sender: mpsc::UnboundedSender<(ExtensionId, PluginEvent)>,
}

impl Default for ExtensionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ExtensionRegistry {
    pub fn new() -> Self {
        let (command_sender, _command_receiver) = mpsc::unbounded_channel();
        let (event_sender, _event_receiver) = mpsc::unbounded_channel();

        Self {
            plugins: RwLock::new(HashMap::new()),
            hooks: HookRegistry::new(),
            skills: SkillRegistry::new(),
            mcp: McpRegistry::new(),
            command_sender,
            event_sender,
        }
    }

    // ─────────────────────────────────────────────────────────────
    // Registration
    // ─────────────────────────────────────────────────────────────

    /// Register a plugin instance
    pub fn register(&self, plugin: Arc<dyn Plugin>) -> Result<ExtensionId, ExtError> {
        let id = ExtensionId(plugin.name().to_string());

        // Check for duplicate
        {
            let plugins = self.plugins.read().unwrap();
            if plugins.contains_key(&id) {
                return Err(ExtError::AlreadyRegistered(id.0));
            }
        }

        // Initialize plugin
        plugin.on_load().map_err(|e| ExtError::LoadFailed(id.0.clone(), e))?;

        let extension = LoadedExtension::new(plugin);
        let metadata = extension.metadata.clone();

        // Register based on type
        match metadata.extension_type {
            ExtensionType::Hook => self.hooks.register_hook(extension.plugin()),
            ExtensionType::Skill => {
                // Convert plugin to skill using the adapter
                let skill = crate::skills::PluginAsSkill::new(extension.plugin().clone());
                self.skills.register(Arc::new(skill) as Arc<dyn crate::skills::Skill>);
            }
            ExtensionType::McpServer => {
                // MCP servers are handled separately
            }
            ExtensionType::Plugin => {}
        }

        let mut plugins = self.plugins.write().unwrap();
        plugins.insert(id.clone(), extension);

        tracing::info!("Registered extension: {} v{}", id, metadata.version);

        Ok(id)
    }

    /// Unregister and unload an extension
    pub fn unregister(&self, id: &ExtensionId) -> Result<(), ExtError> {
        let extension = {
            let mut plugins = self.plugins.write().unwrap();
            plugins.remove(id)
        };

        match extension {
            Some(ext) => {
                ext.plugin().on_unload()
                    .map_err(|e| ExtError::UnloadFailed(id.0.clone(), e))?;
                tracing::info!("Unregistered extension: {}", id);
                Ok(())
            }
            None => Err(ExtError::NotFound(id.0.clone())),
        }
    }

    /// Get all registered plugin metadata
    pub fn list_extensions(&self) -> Vec<PluginMetadata> {
        let plugins = self.plugins.read().unwrap();
        plugins.values().map(|e| e.metadata.clone()).collect()
    }

    /// Get a specific extension by ID
    pub fn get(&self, id: &ExtensionId) -> Option<Arc<dyn Plugin>> {
        let plugins = self.plugins.read().unwrap();
        plugins.get(id).map(|e| e.plugin().clone())
    }

    /// Get all registered commands from all plugins
    pub fn commands(&self) -> Vec<PluginCommand> {
        let plugins = self.plugins.read().unwrap();
        plugins.values()
            .flat_map(|e| e.plugin().commands())
            .collect()
    }

    // ─────────────────────────────────────────────────────────────
    // Event Processing
    // ─────────────────────────────────────────────────────────────

    /// Dispatch event to all interested extensions
    pub fn dispatch_event(&self, event: PluginEvent) -> Vec<PluginAction> {
        let mut actions = Vec::new();

        // Dispatch to hooks first (priority ordering)
        actions.extend(self.hooks.dispatch(event.clone()));

        // Dispatch to plugins
        let plugins = self.plugins.read().unwrap();
        for (_, extension) in plugins.iter() {
            let plugin_actions = extension.plugin().on_event(event.clone());
            actions.extend(plugin_actions);
        }

        actions
    }

    // ─────────────────────────────────────────────────────────────
    // Dynamic Loading
    // ─────────────────────────────────────────────────────────────

    /// Load extensions from configuration directory
    pub async fn load_from_dir(&self, dir: std::path::PathBuf) -> Result<(), ExtError> {
        use tokio::fs;

        if !dir.exists() {
            tracing::debug!("Extension directory does not exist: {:?}", dir);
            return Ok(());
        }

        let mut entries = fs::read_dir(&dir).await
            .map_err(|e| ExtError::IoError(e.to_string()))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| ExtError::IoError(e.to_string()))?
        {
            let path = entry.path();
            if path.extension().map(|e| e == "toml").unwrap_or(false) {
                self.load_manifest(&path).await?;
            }
        }

        Ok(())
    }

    /// Load extension from manifest file
    async fn load_manifest(&self, manifest_path: &std::path::Path) -> Result<(), ExtError> {
        let content = tokio::fs::read_to_string(manifest_path).await
            .map_err(|e| ExtError::IoError(e.to_string()))?;

        let manifest: ExtensionManifest = toml::from_str(&content)
            .map_err(|e| ExtError::ParseError(e.to_string()))?;

        tracing::info!("Loading extension from manifest: {}", manifest.name);

        // For now, we only support loading skills (boxed plugins)
        // MCP servers require spawning processes
        // Hooks are statically registered

        Ok(())
    }
}

/// Manifest for a dynamically loaded extension
#[derive(Debug, serde::Deserialize)]
struct ExtensionManifest {
    name: String,
    version: String,
    #[serde(rename = "type")]
    extension_type: ExtensionType,
    #[serde(default)]
    main: Option<String>,
}

// ─────────────────────────────────────────────────────────────────
// Sub-registries
// ─────────────────────────────────────────────────────────────────

mod sealed {
    pub trait Sealed {}
}

/// Trait for sub-registries (hooks, skills, mcp)
pub trait SubRegistry: sealed::Sealed {
    fn dispatch(&self, event: PluginEvent) -> Vec<PluginAction>;
}
