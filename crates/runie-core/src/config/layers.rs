//! Layered configuration loading using Figment.
//!
//! Configuration precedence (lowest to highest):
//! 1. Defaults (Config::default())
//! 2. Global config (~/.runie/config.toml)
//! 3. Project config (.runie/config.toml)
//! 4. Environment variables (RUNIE_PROVIDER, RUNIE_MODEL, RUNIE_THEME)
//!
//! ## Project-local config restrictions
//!
//! Project-local config (.runie/config.toml) MUST NOT contain sensitive keys
//! like credentials or server endpoints. These belong in the global config or
//! OS keyring. See [`PROJECT_CONFIG_DENYLIST`].

use std::path::{Path, PathBuf};

use figment::providers::{Format, Serialized, Toml};
use figment::Figment;
use toml::Value as TomlValue;

use crate::config::{config_path, Config};

/// Keys that MUST NOT appear in project-local config files (.runie/config.toml).
/// These belong in the global config or OS keyring to prevent accidental
/// credential/endpoint leakage when sharing project directories.
///
/// Covers both top-level keys and nested provider/model keys.
const PROJECT_CONFIG_DENYLIST: &[&str] = &[
    // Credential keys
    "api_key",
    "api-key",
    "apiKey",
    // Server endpoint keys
    "base_url",
    "base-url",
    "baseUrl",
    "openai_base_url",
    // Provider/model config (must be in global config)
    "model_providers",
    "providers",
    "models",
    // Security-sensitive
    "profile",
    "permission_mode",
];

/// Recursively collect all denied keys found in a TOML value.
fn collect_denied_keys(value: &TomlValue, path: &str) -> Vec<String> {
    let mut denied = Vec::new();
    match value {
        TomlValue::Table(map) => {
            for (k, v) in map {
                let full_path = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{path}.{k}")
                };
                if PROJECT_CONFIG_DENYLIST.contains(&k.as_str()) {
                    denied.push(full_path.clone());
                }
                denied.extend(collect_denied_keys(v, &full_path));
            }
        }
        TomlValue::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                let full_path = format!("{path}[{i}]");
                denied.extend(collect_denied_keys(v, &full_path));
            }
        }
        _ => {}
    }
    denied
}

/// Check a TOML file for denied keys and warn about any found.
/// Returns the parsed TOML value on success (or empty table on error).
fn parse_and_check_denylist(path: &Path) -> TomlValue {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("failed to read config file {}: {e}", path.display());
            return TomlValue::Table(Default::default());
        }
    };

    let parsed: TomlValue = match toml::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("failed to parse config file {}: {e}", path.display());
            return TomlValue::Table(Default::default());
        }
    };

    let denied = collect_denied_keys(&parsed, "");
    if !denied.is_empty() {
        tracing::warn!(
            "project config {} contains sensitive keys that should be in global config or keyring: {}",
            path.display(),
            denied.join(", ")
        );
    }

    parsed
}

/// Load configuration from layered sources: defaults → global config →
/// local project config → environment variables.
pub fn load_layers() -> Config {
    load_layers_from_paths(config_path(), PathBuf::from(".runie").join("config.toml"))
}

/// Layered config loader with explicit paths (useful for tests).
pub fn load_layers_from_paths(global: PathBuf, local: PathBuf) -> Config {
    // Build Figment with layered sources
    let mut figment = Figment::new();

    // 1. Defaults
    figment = figment.merge(Serialized::defaults(Config::default()));

    // 2. Global config (only if file exists)
    if global.exists() {
        figment = figment.merge(Toml::file(&global));
    }

    // 3. Project config (only if file exists) — check denylist first
    if local.exists() {
        let _denied = parse_and_check_denylist(&local);
        // Still merge the config (warn but don't reject)
        figment = figment.merge(Toml::file(&local));
    }

    // 4. Environment variables with RUNIE_ prefix.
    // Use Figment's Serialized::default(key, value) to insert env vars.
    // This uses Figment's value-merging machinery (not manual Config field mutation).
    // NOTE: Figment's Env::prefixed() does NOT work here because serde's struct
    // deserialization is case-sensitive while figment preserves the uppercase key
    // from the env var name (PROVIDER vs provider). We use Serialized::default
    // to explicitly set the correctly-cased keys in the Default profile.
    for (env_var, field) in [("RUNIE_PROVIDER", "provider"), ("RUNIE_MODEL", "model"), ("RUNIE_THEME", "theme")] {
        if let Ok(value) = std::env::var(env_var) {
            figment = figment.merge(Serialized::default(field, value));
        }
    }

    // Extract config from Figment
    let config: Config = figment.extract().unwrap_or_else(|e| {
        tracing::warn!("failed to extract config from figment: {}", e);
        Config::default()
    });

    config
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn tmp_file(content: &str) -> (TempDir, PathBuf) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, content).unwrap();
        (dir, path)
    }

    #[test]
    fn denylist_detects_top_level_api_key() {
        let (dir, path) = tmp_file("api_key = \"secret\"\nmodel = \"foo\"\n");
        let denied = collect_denied_keys(
            &toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap(),
            "",
        );
        assert!(
            denied.contains(&"api_key".to_string()),
            "should detect api_key: {denied:?}"
        );
        drop(dir);
    }

    #[test]
    fn denylist_detects_nested_base_url() {
        let (dir, path) = tmp_file("[providers.foo]\nbase_url = \"http://localhost\"\n");
        let denied = collect_denied_keys(
            &toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap(),
            "",
        );
        assert!(
            denied.iter().any(|k| k.contains("base_url")),
            "should detect base_url: {denied:?}"
        );
        drop(dir);
    }

    #[test]
    fn denylist_detects_nested_api_key_in_array() {
        let (dir, path) = tmp_file("[[providers]]\napi_key = \"x\"\nname = \"test\"\n");
        let denied = collect_denied_keys(
            &toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap(),
            "",
        );
        assert!(
            denied.iter().any(|k| k.contains("api_key")),
            "should detect api_key in array: {denied:?}"
        );
        drop(dir);
    }

    #[test]
    fn denylist_allows_safe_keys() {
        let (dir, path) = tmp_file("model = \"foo\"\ntheme = \"dark\"\nprovider = \"minimax\"\n");
        let denied = collect_denied_keys(
            &toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap(),
            "",
        );
        assert!(denied.is_empty(), "should have no denied keys: {denied:?}");
        drop(dir);
    }

    #[test]
    fn parse_and_check_warns_on_denied_keys() {
        let (dir, path) = tmp_file("api_key = \"secret\"\nbase_url = \"http://evil.com\"\n");
        let _ = parse_and_check_denylist(&path);
        // Warning is emitted — we just verify no panic
        drop(dir);
    }

    #[test]
    fn figment_env_overrides_take_precedence() {
        // Verify RUNIE_PROVIDER/MODEL/THEME override config file values via Figment
        let (dir, path) = tmp_file("provider = \"file-provider\"\nmodel = \"file-model\"\ntheme = \"file-theme\"\n");

        // Set env vars that should override file values
        std::env::set_var("RUNIE_PROVIDER", "env-provider");
        std::env::set_var("RUNIE_MODEL", "env-model");
        std::env::set_var("RUNIE_THEME", "env-theme");

        let config = load_layers_from_paths(std::env::temp_dir().join("nonexistent.toml"), path);

        // Clean up env vars
        std::env::remove_var("RUNIE_PROVIDER");
        std::env::remove_var("RUNIE_MODEL");
        std::env::remove_var("RUNIE_THEME");

        // Figment's Env::prefixed("RUNIE_") maps RUNIE_PROVIDER → provider,
        // RUNIE_MODEL → model, RUNIE_THEME → theme (case-insensitive key match)
        assert_eq!(
            config.provider.as_deref(),
            Some("env-provider"),
            "RUNIE_PROVIDER env var should override file value, got {:?}",
            config.provider
        );
        assert_eq!(
            config.model.as_deref(),
            Some("env-model"),
            "RUNIE_MODEL env var should override file value, got {:?}",
            config.model
        );
        assert_eq!(
            config.theme.as_deref(),
            Some("env-theme"),
            "RUNIE_THEME env var should override file value, got {:?}",
            config.theme
        );

        drop(dir);
    }
}
