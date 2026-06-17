//! Layered configuration loading.

use std::path::PathBuf;

use crate::config::{config_path, Config};

/// Load configuration from layered sources: defaults → global config →
/// local project config → environment variables.
pub fn load_layers() -> Config {
    load_layers_from_paths(config_path(), PathBuf::from(".runie").join("config.toml"))
}

/// Layered config loader with explicit paths (useful for tests).
pub fn load_layers_from_paths(global: PathBuf, local: PathBuf) -> Config {
    let mut value =
        toml::Value::try_from(Config::default()).unwrap_or_else(|_| toml::Value::Table(toml::map::Map::new()));

    if let Ok(text) = std::fs::read_to_string(&global) {
        if let Ok(v) = toml::from_str::<toml::Value>(&text) {
            merge_toml_values(&mut value, v);
        }
    }
    if let Ok(text) = std::fs::read_to_string(&local) {
        if let Ok(v) = toml::from_str::<toml::Value>(&text) {
            merge_toml_values(&mut value, v);
        }
    }
    apply_env_overrides(&mut value);
    toml::from_str(&toml::to_string(&value).unwrap_or_default()).unwrap_or_default()
}

/// Recursively merge `other` into `base`. Tables are merged; scalar values are
/// replaced by `other`.
fn merge_toml_values(base: &mut toml::Value, other: toml::Value) {
    let (base_table, other_table) = match (base.as_table_mut(), other.as_table()) {
        (Some(b), Some(o)) => (b, o),
        _ => {
            *base = other;
            return;
        }
    };
    for (key, value) in other_table.iter() {
        match base_table.get_mut(key) {
            Some(existing) => merge_toml_values(existing, value.clone()),
            None => {
                base_table.insert(key.clone(), value.clone());
            }
        }
    }
}

/// Apply environment variable overrides to a TOML config value.
fn apply_env_overrides(value: &mut toml::Value) {
    let table = match value.as_table_mut() {
        Some(t) => t,
        None => return,
    };
    if let Ok(provider) = std::env::var("RUNIE_PROVIDER") {
        table.insert("provider".into(), toml::Value::String(provider));
    }
    if let Ok(model) = std::env::var("RUNIE_MODEL") {
        table.insert("model".into(), toml::Value::String(model));
    }
    if let Ok(theme) = std::env::var("RUNIE_THEME") {
        table.insert("theme".into(), toml::Value::String(theme));
    }
}
