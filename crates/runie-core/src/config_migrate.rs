//! Config migration framework — versioned config with upgrade paths.
//!
//! On load, if the config file lacks a `version` field or has an older version,
//! the migrator applies incremental transformations and writes the result back.

use std::path::{Path, PathBuf};

pub const CURRENT_CONFIG_VERSION: u32 = 3;

/// Migrate a parsed TOML value to the current version.
/// Returns `Ok(true)` if mutations were applied.
pub fn migrate(config: &mut toml::Value) -> anyhow::Result<bool> {
    migrate_with_path(config, None)
}

/// Migrate with a specific config path (useful for testing).
pub fn migrate_with_path(config: &mut toml::Value, config_path: Option<std::path::PathBuf>) -> anyhow::Result<bool> {
    let version = config
        .get("version")
        .and_then(|v| v.as_integer())
        .unwrap_or(0) as u32;

    if version >= CURRENT_CONFIG_VERSION {
        return Ok(false);
    }

    if version < 1 {
        v0_to_v1(config)?;
    }
    if version < 2 {
        v1_to_v2(config)?;
    }
    if version < 3 {
        v2_to_v3(config, config_path)?;
    }

    if let Some(map) = config.as_table_mut() {
        map.insert(
            "version".into(),
            toml::Value::Integer(CURRENT_CONFIG_VERSION as i64),
        );
    }
    Ok(true)
}

/// v2 → v3: migrate `keybindings.json` to `[keybindings]` table in config.toml.
fn v2_to_v3(config: &mut toml::Value, config_path: Option<std::path::PathBuf>) -> anyhow::Result<()> {
    let map = config
        .as_table_mut()
        .ok_or_else(|| anyhow::anyhow!("config must be a table"))?;

    if map.get("keybindings").is_some() {
        // Already has keybindings table — nothing to migrate
        return Ok(());
    }

    // Check for legacy keybindings.json relative to config path
    let cfg_path = config_path.unwrap_or_else(crate::config::config_path);
    let json_path = cfg_path.with_file_name("keybindings.json");
    if json_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&json_path) {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(obj) = value.as_object() {
                    let mut kb_table = toml::value::Table::new();
                    for (k, v) in obj {
                        if let Some(s) = v.as_str() {
                            kb_table.insert(k.clone().into(), toml::Value::String(s.to_string()));
                        }
                    }
                    map.insert("keybindings".into(), toml::Value::Table(kb_table));
                    // Rename JSON file to .bak
                    let bak_path = json_path.with_extension("json.bak");
                    let _ = std::fs::rename(&json_path, &bak_path);
                }
            }
        }
    }

    Ok(())
}

/// Backup the config file before migration.
/// Returns the path of the backup file.
pub fn backup_config(path: &Path) -> anyhow::Result<PathBuf> {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("config");
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("toml");
    let backup_name = format!("{}_backup.{}", stem, ext);
    let backup_path = path.with_file_name(backup_name);
    std::fs::copy(path, &backup_path)?;
    Ok(backup_path)
}

/// v0 → v1: move top-level `model` into `[models].default` if not already set.
fn v0_to_v1(config: &mut toml::Value) -> anyhow::Result<()> {
    let map = config
        .as_table_mut()
        .ok_or_else(|| anyhow::anyhow!("config must be a table"))?;

    if let Some(model) = map.remove("model") {
        let models_table = map
            .entry("models")
            .or_insert_with(|| toml::Value::Table(toml::value::Table::new()))
            .as_table_mut()
            .ok_or_else(|| anyhow::anyhow!("models must be a table"))?;
        if !models_table.contains_key("default") {
            models_table.insert("default".into(), model);
        }
    }

    Ok(())
}

/// v1 → v2: ensure `[model_providers]` exists as a table.
fn v1_to_v2(config: &mut toml::Value) -> anyhow::Result<()> {
    let map = config
        .as_table_mut()
        .ok_or_else(|| anyhow::anyhow!("config must be a table"))?;

    if !map.contains_key("model_providers") {
        map.insert(
            "model_providers".into(),
            toml::Value::Table(toml::value::Table::new()),
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrate_v0_to_v1() {
        let mut config: toml::Value = toml::from_str(
            r#"
provider = "openai"
model = "gpt-4"
theme = "dracula"
"#,
        )
        .unwrap();

        let changed = migrate(&mut config).unwrap();
        assert!(changed);
        assert_eq!(
            config["version"].as_integer(),
            Some(CURRENT_CONFIG_VERSION as i64)
        );
        assert_eq!(config["provider"].as_str(), Some("openai"));
        assert_eq!(config["models"]["default"].as_str(), Some("gpt-4"));
        assert!(config.get("model").is_none());
    }

    #[test]
    fn migrate_v1_to_v2() {
        let mut config: toml::Value = toml::from_str(
            r#"
version = 1
provider = "openai"

[models]
default = "gpt-4"
"#,
        )
        .unwrap();

        let changed = migrate(&mut config).unwrap();
        assert!(changed);
        assert_eq!(
            config["version"].as_integer(),
            Some(CURRENT_CONFIG_VERSION as i64)
        );
        assert!(config["model_providers"].is_table());
    }

    #[test]
    fn migrate_noop_when_current() {
        let mut config: toml::Value = toml::from_str(&format!(
            r#"
version = {}
provider = "openai"
"#,
            CURRENT_CONFIG_VERSION
        ))
        .unwrap();

        let changed = migrate(&mut config).unwrap();
        assert!(!changed);
    }

    #[test]
    fn backup_created() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "provider = 'openai'\n").unwrap();

        let backup = backup_config(&path).unwrap();
        assert!(backup.exists());
        assert_eq!(
            backup.file_name().unwrap().to_str().unwrap(),
            "config_backup.toml"
        );
    }
}
