//! Pure plugin manifests and registry data. Loading and lifecycle ownership
//! stay at the application boundary; this module only validates and stores
//! declarative extension metadata.

use std::collections::BTreeMap;
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
}
