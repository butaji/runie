//! Pure plugin manifests and registry data. Loading and lifecycle ownership
//! stay at the application boundary; this module only validates and stores
//! declarative extension metadata.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub commands: Vec<String>,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub hooks: Vec<String>,
}

impl PluginManifest {
    pub fn validate(&self) -> Result<(), String> {
        validate_identifier("plugin name", &self.name)?;
        if self.version.trim().is_empty() {
            return Err("plugin version must not be empty".into());
        }
        for (kind, values) in [
            ("command", &self.commands),
            ("tool", &self.tools),
            ("hook", &self.hooks),
        ] {
            for value in values {
                validate_identifier(kind, value)?;
            }
        }
        Ok(())
    }

    pub fn from_json(input: &str) -> Result<Self, String> {
        let manifest: Self = serde_json::from_str(input)
            .map_err(|error| format!("invalid plugin manifest JSON: {error}"))?;
        manifest.validate()?;
        Ok(manifest)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginPackage {
    pub root: PathBuf,
    pub manifest: PluginManifest,
}

pub fn load_manifest(path: impl AsRef<Path>) -> Result<PluginManifest, String> {
    let path = path.as_ref();
    let input = std::fs::read_to_string(path)
        .map_err(|error| format!("read plugin manifest {}: {error}", path.display()))?;
    PluginManifest::from_json(&input)
}

pub fn discover_packages(root: impl AsRef<Path>) -> Result<Vec<PluginPackage>, String> {
    let root = root.as_ref();
    let mut packages = Vec::new();
    let entries = std::fs::read_dir(root)
        .map_err(|error| format!("read plugin directory {}: {error}", root.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("read plugin directory entry: {error}"))?;
        if !entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_dir()
        {
            continue;
        }
        let package_root = entry.path();
        let manifest_path = package_root.join("plugin.json");
        if manifest_path.is_file() {
            packages.push(PluginPackage {
                root: package_root,
                manifest: load_manifest(manifest_path)?,
            });
        }
    }
    packages.sort_by(|left, right| left.manifest.name.cmp(&right.manifest.name));
    Ok(packages)
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PluginRegistry {
    manifests: BTreeMap<String, PluginManifest>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PluginLifecycleEvent {
    Activated { name: String },
    Deactivated { name: String },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PluginRuntimeSnapshot {
    pub active: BTreeSet<String>,
}

#[path = "plugin_runtime.rs"]
mod runtime;
pub use runtime::{
    reduce_plugin_runtime, PluginRuntimeEvent, PluginRuntimeState, PluginRuntimeStatus,
};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PluginInstallationEvent {
    Installed { name: String, root: PathBuf },
    Uninstalled { name: String },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PluginInstallationSnapshot {
    pub roots: BTreeMap<String, PathBuf>,
}

pub fn reduce_plugin_installation(
    registry: &PluginRegistry,
    snapshot: &mut PluginInstallationSnapshot,
    event: PluginInstallationEvent,
) -> Result<(), String> {
    match event {
        PluginInstallationEvent::Installed { name, root } => {
            if registry.get(&name).is_none() {
                return Err(format!("plugin is not registered: {name}"));
            }
            if root.as_os_str().is_empty() {
                return Err(format!("plugin root is empty: {name}"));
            }
            if snapshot.roots.insert(name.clone(), root).is_some() {
                return Err(format!("plugin is already installed: {name}"));
            }
        }
        PluginInstallationEvent::Uninstalled { name } => {
            if snapshot.roots.remove(&name).is_none() {
                return Err(format!("plugin is not installed: {name}"));
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ActivePluginCapability {
    pub plugin: String,
    pub kind: String,
    pub name: String,
}

/// Materialize active manifest declarations for hosts and renderers.
/// Capability execution stays outside this pure projection boundary.
pub fn active_plugin_capabilities(
    registry: &PluginRegistry,
    snapshot: &PluginRuntimeSnapshot,
) -> Result<Vec<ActivePluginCapability>, String> {
    let mut capabilities = Vec::new();
    for plugin in &snapshot.active {
        let manifest = registry
            .get(plugin)
            .ok_or_else(|| format!("plugin is not registered: {plugin}"))?;
        for (kind, names) in [
            ("command", &manifest.commands),
            ("tool", &manifest.tools),
            ("hook", &manifest.hooks),
        ] {
            capabilities.extend(names.iter().map(|name| ActivePluginCapability {
                plugin: plugin.clone(),
                kind: kind.to_owned(),
                name: name.clone(),
            }));
        }
    }
    Ok(capabilities)
}

pub fn active_installed_plugin_capabilities(
    registry: &PluginRegistry,
    installation: &PluginInstallationSnapshot,
    runtime: &PluginRuntimeSnapshot,
) -> Result<Vec<ActivePluginCapability>, String> {
    for name in &runtime.active {
        if !installation.roots.contains_key(name) {
            return Err(format!("active plugin is not installed: {name}"));
        }
    }
    active_plugin_capabilities(registry, runtime)
}

/// Pure plugin lifecycle reducer. Loading, hook execution, and unloading are
/// host concerns; this projection makes activation state replayable and
/// rejects events for unknown or already-settled plugins.
pub fn reduce_plugin_lifecycle(
    registry: &PluginRegistry,
    snapshot: &mut PluginRuntimeSnapshot,
    event: PluginLifecycleEvent,
) -> Result<(), String> {
    let name = match &event {
        PluginLifecycleEvent::Activated { name } | PluginLifecycleEvent::Deactivated { name } => {
            name
        }
    };
    if registry.get(name).is_none() {
        return Err(format!("plugin is not registered: {name}"));
    }
    match event {
        PluginLifecycleEvent::Activated { name } if !snapshot.active.insert(name.clone()) => {
            Err(format!("plugin is already active: {name}"))
        }
        PluginLifecycleEvent::Deactivated { name } if !snapshot.active.remove(&name) => {
            Err(format!("plugin is not active: {name}"))
        }
        _ => Ok(()),
    }
}

impl PluginRegistry {
    pub fn register(&mut self, manifest: PluginManifest) -> Result<(), String> {
        manifest.validate()?;
        if self.manifests.contains_key(&manifest.name) {
            return Err(format!("plugin is already registered: {}", manifest.name));
        }
        self.manifests.insert(manifest.name.clone(), manifest);
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&PluginManifest> {
        self.manifests.get(name)
    }

    pub fn manifests(&self) -> impl Iterator<Item = &PluginManifest> {
        self.manifests.values()
    }
}

fn validate_identifier(kind: &str, value: &str) -> Result<(), String> {
    let valid = !value.is_empty()
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'));
    if valid {
        Ok(())
    } else {
        Err(format!("{kind} identifier is invalid: {value:?}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> PluginManifest {
        PluginManifest {
            name: "sample-plugin".into(),
            version: "1.0.0".into(),
            commands: vec!["format".into()],
            tools: vec!["inspect".into()],
            hooks: vec!["after_turn".into()],
        }
    }

    #[test]
    fn registry_keeps_valid_plugin_data() {
        let mut registry = PluginRegistry::default();
        registry.register(manifest()).unwrap();
        assert_eq!(registry.get("sample-plugin").unwrap().tools, ["inspect"]);
        assert_eq!(registry.manifests().count(), 1);
    }

    #[test]
    fn registry_rejects_invalid_and_duplicate_manifests() {
        let mut registry = PluginRegistry::default();
        let mut invalid = manifest();
        invalid.commands = vec!["bad command".into()];
        assert!(registry.register(invalid).is_err());
        registry.register(manifest()).unwrap();
        assert!(registry
            .register(manifest())
            .unwrap_err()
            .contains("already"));
    }

    #[test]
    fn manifest_json_is_validated_before_registration() {
        let json = serde_json::to_string(&manifest()).unwrap();
        assert_eq!(PluginManifest::from_json(&json).unwrap(), manifest());
        assert!(PluginManifest::from_json(r#"{"name":"bad name","version":"1"}"#).is_err());
    }

    #[test]
    fn discovery_loads_sorted_plugin_packages() {
        let root = std::env::temp_dir().join(format!("runie-plugins-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("zeta")).unwrap();
        std::fs::create_dir_all(root.join("alpha")).unwrap();
        std::fs::write(
            root.join("zeta/plugin.json"),
            serde_json::to_vec(&PluginManifest {
                name: "zeta".into(),
                ..manifest()
            })
            .unwrap(),
        )
        .unwrap();
        std::fs::write(
            root.join("alpha/plugin.json"),
            serde_json::to_vec(&PluginManifest {
                name: "alpha".into(),
                ..manifest()
            })
            .unwrap(),
        )
        .unwrap();
        let packages = discover_packages(&root).unwrap();
        assert_eq!(
            packages
                .iter()
                .map(|package| package.manifest.name.as_str())
                .collect::<Vec<_>>(),
            ["alpha", "zeta"]
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn lifecycle_replay_is_deterministic_and_validated() {
        let mut registry = PluginRegistry::default();
        registry.register(manifest()).unwrap();
        let mut state = PluginRuntimeSnapshot::default();
        reduce_plugin_lifecycle(
            &registry,
            &mut state,
            PluginLifecycleEvent::Activated {
                name: "sample-plugin".into(),
            },
        )
        .unwrap();
        assert!(reduce_plugin_lifecycle(
            &registry,
            &mut state,
            PluginLifecycleEvent::Activated {
                name: "sample-plugin".into(),
            },
        )
        .is_err());
        reduce_plugin_lifecycle(
            &registry,
            &mut state,
            PluginLifecycleEvent::Deactivated {
                name: "sample-plugin".into(),
            },
        )
        .unwrap();
        assert!(state.active.is_empty());
    }

    #[test]
    fn active_capabilities_are_sorted_and_data_only() {
        let mut registry = PluginRegistry::default();
        registry.register(manifest()).unwrap();
        let mut state = PluginRuntimeSnapshot::default();
        reduce_plugin_lifecycle(
            &registry,
            &mut state,
            PluginLifecycleEvent::Activated {
                name: "sample-plugin".into(),
            },
        )
        .unwrap();

        assert_eq!(
            active_plugin_capabilities(&registry, &state).unwrap(),
            vec![
                ActivePluginCapability {
                    plugin: "sample-plugin".into(),
                    kind: "command".into(),
                    name: "format".into(),
                },
                ActivePluginCapability {
                    plugin: "sample-plugin".into(),
                    kind: "tool".into(),
                    name: "inspect".into(),
                },
                ActivePluginCapability {
                    plugin: "sample-plugin".into(),
                    kind: "hook".into(),
                    name: "after_turn".into(),
                },
            ]
        );
    }

    #[test]
    fn installation_replay_tracks_owned_roots_and_rejects_duplicates() {
        let mut registry = PluginRegistry::default();
        registry.register(manifest()).unwrap();
        let mut snapshot = PluginInstallationSnapshot::default();
        reduce_plugin_installation(
            &registry,
            &mut snapshot,
            PluginInstallationEvent::Installed {
                name: "sample-plugin".into(),
                root: PathBuf::from("/plugins/sample"),
            },
        )
        .unwrap();
        assert!(reduce_plugin_installation(
            &registry,
            &mut snapshot,
            PluginInstallationEvent::Installed {
                name: "sample-plugin".into(),
                root: PathBuf::from("/plugins/other"),
            },
        )
        .is_err());
        reduce_plugin_installation(
            &registry,
            &mut snapshot,
            PluginInstallationEvent::Uninstalled {
                name: "sample-plugin".into(),
            },
        )
        .unwrap();
        assert!(snapshot.roots.is_empty());
    }

    #[test]
    fn active_capabilities_require_an_installed_plugin_root() {
        let mut registry = PluginRegistry::default();
        registry.register(manifest()).unwrap();
        let runtime = PluginRuntimeSnapshot {
            active: ["sample-plugin".into()].into_iter().collect(),
        };
        assert!(active_installed_plugin_capabilities(
            &registry,
            &PluginInstallationSnapshot::default(),
            &runtime
        )
        .is_err());
    }
}
