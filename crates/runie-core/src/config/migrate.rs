//! Config migration framework — versioned config with upgrade paths.
//!
//! On load, if the config file lacks a `version` field or has an older version,
//! the migrator applies incremental transformations and writes the result back.

use std::path::{Path, PathBuf};

pub const CURRENT_CONFIG_VERSION: u32 = 4;

/// Migrate a parsed TOML value to the current version.
/// Returns `Ok(true)` if mutations were applied.
pub fn migrate(config: &mut toml::Value) -> anyhow::Result<bool> {
    migrate_with_path(config, None)
}

/// Migrate with a specific config path (useful for testing).
pub fn migrate_with_path(
    config: &mut toml::Value,
    config_path: Option<std::path::PathBuf>,
) -> anyhow::Result<bool> {
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
    if version < 4 {
        v3_to_v4(config)?;
    }

    if let Some(map) = config.as_table_mut() {
        map.insert(
            "version".into(),
            toml::Value::Integer(CURRENT_CONFIG_VERSION as i64),
        );
    }
    Ok(true)
}

/// v3 → v4: migrate plaintext `api_key` values.
///
/// Iterates through `[model_providers.*]` and removes plaintext `api_key`
/// from the config file. When the `keyring` feature is enabled, keys are
/// also stored in the OS keyring. When disabled, keys are simply removed
/// from config (resolution falls back to env variables).
fn v3_to_v4(config: &mut toml::Value) -> anyhow::Result<()> {
    #[cfg(feature = "keyring")]
    #[allow(unused_imports)]
    use crate::auth;

    let map = config
        .as_table_mut()
        .ok_or_else(|| anyhow::anyhow!("config must be a table"))?;

    let providers = match map.get_mut("model_providers") {
        Some(toml::Value::Table(t)) => t,
        _ => return Ok(()),
    };

    for (name, provider_value) in providers.iter_mut() {
        let provider_map = match provider_value.as_table_mut() {
            Some(m) => m,
            None => continue,
        };

        let api_key = match provider_map.get("api_key") {
            Some(toml::Value::String(s)) if !s.is_empty() => Some(s.clone()),
            _ => None,
        };

        // If there's a non-empty api_key, store it in keyring (requires keyring feature)
        #[cfg(feature = "keyring")]
        {
            if let Some(key) = api_key {
                if let Err(e) = crate::auth::set_keyring_value(name, &key) {
                    tracing::warn!("failed to migrate api_key for {} to keyring: {}", name, e);
                } else {
                    tracing::info!("migrated api_key for {} to keyring", name);
                }
            }
        }

        // Always remove api_key from config (resolution uses keyring/env)
        provider_map.remove("api_key");
    }

    Ok(())
}

/// v2 → v3: migrate `keybindings.json` to `[keybindings]` table in config.toml.
fn v2_to_v3(
    config: &mut toml::Value,
    config_path: Option<std::path::PathBuf>,
) -> anyhow::Result<()> {
    let map = config
        .as_table_mut()
        .ok_or_else(|| anyhow::anyhow!("config must be a table"))?;

    if map.get("keybindings").is_some() {
        return Ok(());
    }

    let cfg_path = config_path.unwrap_or_else(super::config_path);
    let json_path = cfg_path.with_file_name("keybindings.json");
    if let Some(kb_table) = read_legacy_keybindings(&json_path) {
        map.insert("keybindings".into(), toml::Value::Table(kb_table));
    }

    Ok(())
}

fn read_legacy_keybindings(json_path: &std::path::Path) -> Option<toml::value::Table> {
    if !json_path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(json_path).ok()?;
    let value = serde_json::from_str::<serde_json::Value>(&content).ok()?;
    let obj = value.as_object()?;
    let mut kb_table = toml::value::Table::new();
    for (k, v) in obj {
        if let Some(s) = v.as_str() {
            kb_table.insert(k.clone(), toml::Value::String(s.to_owned()));
        }
    }
    if !kb_table.is_empty() {
        let bak_path = json_path.with_extension("json.bak");
        let _ = std::fs::rename(json_path, bak_path);
    }
    Some(kb_table)
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
    use crate::config::Config;

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
    fn migrate_v3_to_v4_parses_back_to_config() {
        // This test verifies that after migration, the TOML can be parsed
        // back into a Config struct correctly.
        let mut config: toml::Value = toml::from_str(
            r#"
version = 3
provider = "openai"

[model_providers.openai]
type = "openai"
base_url = "https://api.openai.com/v1"
api_key = "sk-plaintext"
models = ["gpt-4", "gpt-3.5-turbo"]
"#,
        )
        .unwrap();

        // Run migration
        let changed = migrate(&mut config).unwrap();
        assert!(changed);

        // Serialize and parse back
        let s = toml::to_string(&config).unwrap();

        // Parse into Config
        let parsed_config: Config = toml::from_str(&s).unwrap();

        // Verify provider is present
        assert!(
            !parsed_config.model_providers.is_empty(),
            "model_providers should not be empty"
        );
        assert!(parsed_config.model_providers.contains_key("openai"));
        let provider = parsed_config.model_providers.get("openai").unwrap();
        assert_eq!(provider.base_url, "https://api.openai.com/v1");
    }

    #[test]
    fn migrate_v3_to_v4_removes_plaintext_api_keys() {
        let mut config: toml::Value = toml::from_str(
            r#"
version = 3
provider = "openai"

[model_providers.openai]
base_url = "https://api.openai.com/v1"
api_key = "sk-plaintext"
models = ["gpt-4", "gpt-3.5-turbo"]

[model_providers.anthropic]
base_url = "https://api.anthropic.com"
api_key = ""
models = ["claude-3"]
"#,
        )
        .unwrap();

        // Migration should run
        let changed = migrate(&mut config).unwrap();
        assert!(changed, "migration should make changes");
        assert_eq!(
            config["version"].as_integer(),
            Some(CURRENT_CONFIG_VERSION as i64)
        );

        // openai has non-empty api_key - should be removed (migrated to keyring)
        let openai = &config["model_providers"]["openai"];
        assert!(
            openai.get("api_key").is_none(),
            "api_key should be removed after migration"
        );
        assert_eq!(
            openai["base_url"].as_str(),
            Some("https://api.openai.com/v1")
        );

        // anthropic has empty api_key - should also be removed (no keyring needed)
        let anthropic = &config["model_providers"]["anthropic"];
        assert!(
            anthropic.get("api_key").is_none(),
            "empty api_key should also be removed"
        );
    }

    #[test]
    fn migrate_v3_to_v4_no_providers() {
        // Config without model_providers should not fail
        let mut config: toml::Value = toml::from_str(
            r#"
version = 3
provider = "openai"
"#,
        )
        .unwrap();

        let changed = migrate(&mut config).unwrap();
        assert!(changed);
        assert_eq!(
            config["version"].as_integer(),
            Some(CURRENT_CONFIG_VERSION as i64)
        );
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
