use std::collections::HashMap;

use serde::Deserialize;

/// A binding entry parsed from the YAML resource.
#[derive(Debug, Deserialize)]
struct BindingEntry {
    combo: String,
    event: String,
    #[serde(default)]
    condition: Option<String>,
}

#[derive(Debug, Deserialize)]
struct KeybindingsYaml {
    bindings: Vec<BindingEntry>,
}

/// Default bindings loaded from resources/keybindings/default.yaml.
/// This replaces the hard-coded DEFAULT_BINDINGS array.
pub fn default_keybindings() -> HashMap<String, String> {
    let yaml_content = include_str!("../../resources/keybindings/default.yaml");
    let data: KeybindingsYaml = serde_yaml::from_str(yaml_content).unwrap_or_else(|e| {
        tracing::warn!("Failed to parse default keybindings YAML: {}, using empty map", e);
        KeybindingsYaml { bindings: vec![] }
    });

    let mut map = HashMap::new();
    for binding in data.bindings {
        // Check platform conditions
        if let Some(condition) = &binding.condition {
            let condition = condition.trim();
            let passes = match condition {
                "windows" => cfg!(target_os = "windows"),
                "!windows" => !cfg!(target_os = "windows"),
                _ => true,
            };
            if !passes {
                continue;
            }
        }
        map.insert(binding.combo, binding.event);
    }
    map
}
