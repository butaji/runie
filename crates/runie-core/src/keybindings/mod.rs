//! Configurable keybindings module.
//!
//! Keybindings are loaded from the `[keybindings]` table in `~/.runie/config.toml`
//! and merged with defaults from this module. The legacy `keybindings.json` is
//! auto-migrated to `config.toml` by the config migration system (v2→v3).

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::Event;

mod defaults;
#[cfg(test)]
mod tests;

pub use defaults::{default_keybindings, VALID_KEYS};

/// Parse a key combination string to components
/// Examples: "ctrl+c", "alt+enter", "shift+up"
#[cfg(test)]
fn parse_key_combo(combo: &str) -> (Vec<String>, String) {
    let lower = combo.to_lowercase();
    let parts: Vec<&str> = lower.split('+').collect();
    if parts.is_empty() {
        return (vec![], String::new());
    }
    let key = parts[parts.len() - 1].to_owned();
    let modifiers: Vec<String> = parts[..parts.len() - 1]
        .iter()
        .map(|s| s.to_string())
        .collect();
    (modifiers, key)
}

/// Load keybindings from an optional config, falling back to defaults.
/// User entries in `config.keybindings` override defaults; all other defaults remain.
pub fn load_keybindings(config: Option<&crate::config::Config>) -> HashMap<String, String> {
    match config {
        Some(cfg) => merged_keybindings(cfg),
        None => {
            // Legacy path: try loading from keybindings.json if it exists
            let json_path = default_keybindings_path()
                .unwrap_or_else(|| PathBuf::from("/tmp/runie_keybindings.json"));
            if json_path.exists() {
                match fs::read_to_string(&json_path) {
                    Ok(content) => parse_keybindings_json(&content).unwrap_or_else(|e| {
                        tracing::warn!("Failed to parse keybindings: {}, using defaults", e);
                        default_keybindings()
                    }),
                    Err(e) => {
                        tracing::warn!("Failed to read keybindings file: {}, using defaults", e);
                        default_keybindings()
                    }
                }
            } else {
                default_keybindings()
            }
        }
    }
}

/// Merge user keybinding overrides with defaults.
/// User entries take precedence; unspecified keys use defaults.
pub fn merged_keybindings(config: &crate::config::Config) -> HashMap<String, String> {
    let mut bindings = default_keybindings();
    for (combo, event) in &config.keybindings {
        bindings.insert(combo.to_lowercase(), event.clone());
    }
    bindings
}

/// Parse keybindings from JSON string
pub fn parse_keybindings_json(content: &str) -> Result<HashMap<String, String>> {
    let value: serde_json::Value =
        serde_json::from_str(content).context("parse keybindings JSON")?;

    let obj = value.as_object().context("keybindings must be an object")?;

    let mut bindings = default_keybindings(); // Start with defaults

    for (key, val) in obj {
        let event_name = val
            .as_str()
            .context(format!("binding value for '{}' must be a string", key))?
            .to_owned();
        bindings.insert(key.to_lowercase(), event_name);
    }

    Ok(bindings)
}

/// Convert an event name string to an Event variant.
/// Supports simple names (e.g. "Quit", "Submit") and Input prefix (e.g. "Input:\t").
pub fn event_from_name(name: &str) -> Option<Event> {
    Event::from_name(name)
}

/// Get default keybindings file path
pub fn default_keybindings_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("runie").join("keybindings.json"))
}

/// Validate that a key combo string is well-formed
pub fn validate_key_combo(combo: &str) -> bool {
    let parts: Vec<&str> = combo.split('+').collect();
    if parts.is_empty() || parts.len() > 3 {
        return false;
    }
    let key = parts[parts.len() - 1];
    VALID_KEYS.contains(&key)
}
