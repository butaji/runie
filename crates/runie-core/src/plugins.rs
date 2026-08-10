//! Pure plugin manifests and registry data. Loading and lifecycle ownership
//! stay at the application boundary; this module only validates and stores
//! declarative extension metadata.

use std::collections::BTreeMap;

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
}
