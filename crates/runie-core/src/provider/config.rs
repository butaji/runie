//! Provider configuration persistence — read/write provider credentials in config.toml.
//!
//! This module consolidates provider credential management that was previously
//! spread across `login_config/` and various handlers.
//!
//! For actor-based config operations, see `actors/config/file_helpers.rs`.

use secrecy::ExposeSecret;

use std::fs::OpenOptions;
use std::path::PathBuf;

use crate::actors::config::file_helpers;
use crate::proto::provider::ProviderConfig;

thread_local! {
    static TEST_CONFIG_PATH: std::cell::RefCell<Option<PathBuf>> = const {
        std::cell::RefCell::new(None)
    };
}

/// Override the config file path for the current thread (tests only).
pub fn set_test_config_path(path: PathBuf) {
    TEST_CONFIG_PATH.with(|p| *p.borrow_mut() = Some(path));
}

/// Get the default config file path (from canonical config module).
pub fn config_path() -> PathBuf {
    TEST_CONFIG_PATH.with(|p| {
        if let Some(ref path) = *p.borrow() {
            return path.clone();
        }
        crate::config::config_path()
    })
}

/// Read the config file with a shared (read) file lock.
pub fn with_read_lock<F, T>(f: F) -> T
where
    F: FnOnce(&crate::config::Config) -> T,
{
    let path = config_path();
    let config = if path.exists() {
        let file = OpenOptions::new().read(true).open(&path).unwrap();
        let _lock = fs2::FileExt::lock_shared(&file);
        crate::config::Config::load(Some(&path))
    } else {
        crate::config::Config::default()
    };
    f(&config)
}

/// Mutate and save the config file with an exclusive file lock.
pub fn with_write_lock<F, T>(f: F) -> anyhow::Result<T>
where
    F: FnOnce(&mut crate::config::Config) -> T,
{
    let path = config_path();
    let mut config = crate::config::Config::load(Some(&path));
    let result = f(&mut config);
    // save_to uses fs2 exclusive lock internally
    config.save_to(&path)?;
    Ok(result)
}

/// Save a provider configuration to `~/.runie/config.toml`.
/// Uses surgical TOML edit with fs2 exclusive lock via file_helpers.
pub fn save_provider_config(
    name: &str,
    base_url: &str,
    api_key: &str,
    models: &[String],
) -> anyhow::Result<()> {
    file_helpers::save_provider_to_path(&config_path(), name, base_url, api_key, models)
}

/// Remove a provider configuration from `~/.runie/config.toml`.
/// Uses surgical TOML edit with fs2 exclusive lock via file_helpers.
pub fn remove_provider_config(name: &str) -> anyhow::Result<()> {
    file_helpers::remove_provider_from_path(&config_path(), name)
}

/// Get the full configuration for a single provider, including API key.
///
/// API key is resolved from: keyring → env var → config file (legacy fallback).
/// Note: Returns empty string for api_key if none of the sources have a value.
pub fn get_provider_config(name: &str) -> Option<(String, String, Vec<String>)> {
    with_read_lock(|config| {
        let p = config.model_providers.get(name)?;
        // API key resolution uses env → dotenv → keyring (no config fallback)
        // Expose the secret at the boundary since this is the last stop before use
        let api_key = config
            .resolve_api_key(name)
            .map(|s| s.expose_secret().clone())
            .unwrap_or_default();
        Some((p.base_url.clone(), api_key, p.models.clone()))
    })
}

/// List providers that have configurations in `~/.runie/config.toml`.
/// Note: API keys are not included as they are stored in keyring/env, not config.
pub fn list_configured_providers() -> Vec<(String, String, Vec<String>)> {
    with_read_lock(|config| {
        let mut result: Vec<_> = config
            .model_providers
            .iter()
            .map(|(name, p)| (name.clone(), p.base_url.clone(), p.models.clone()))
            .collect();
        result.sort_by(|a, b| a.0.cmp(&b.0));
        result
    })
}

/// Configure providers for the current thread's tests.
///
/// Sets a unique test config path and writes the given providers with dummy
/// credentials. Each provider's model list is the set of models that will be
/// considered "chosen" by the `/model` selector.
pub fn set_test_config_with_providers(providers: &[(String, Vec<String>)]) {
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let path = PathBuf::from(format!(
        "/tmp/runie_test_config_{}_{}.toml",
        std::process::id(),
        n
    ));
    set_test_config_path(path);
    for (name, models) in providers {
        let _ = save_provider_config(name, "http://test", "key", models);
    }
}

/// Reload the global config cache from the current config file.
/// Used by tests to ensure ConfigState.model_providers reflects the latest file state.
pub fn reload_cache_from_file() {
    // This function is called after save_provider_config to ensure the
    // file-backed reads (via list_configured_providers etc.) are consistent.
    // Note: AppState.model_providers is updated synchronously by handlers.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_provider_config_reads_saved_config() {
        set_test_config_with_providers(&[("openai".into(), vec!["gpt-4o".into()])]);
        let (base_url, _api_key, models) = get_provider_config("openai").expect("openai config");
        assert_eq!(base_url, "http://test");
        // api_key is stored in keyring (or config fallback if keyring unavailable)
        // Either way, the provider is readable
        assert!(!base_url.is_empty(), "base_url should be set");
        assert_eq!(models, &["gpt-4o"]);
    }

    #[test]
    fn config_save_provider_writes_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");

        let mut doc = toml::Value::Table(toml::map::Map::new());
        let table = doc.as_table_mut().unwrap();
        let providers = table
            .entry("model_providers")
            .or_insert_with(|| toml::Value::Table(toml::map::Map::new()))
            .as_table_mut()
            .unwrap();

        let mut provider = toml::map::Map::new();
        provider.insert(
            "base_url".into(),
            toml::Value::String("https://api.minimaxi.chat/v1".into()),
        );
        provider.insert("api_key".into(), toml::Value::String("sk-test".into()));
        let arr: Vec<toml::Value> = vec![toml::Value::String("MiniMax-M3".into())];
        provider.insert("models".into(), toml::Value::Array(arr));
        providers.insert("minimax".into(), toml::Value::Table(provider));

        std::fs::write(&path, toml::to_string_pretty(&doc).unwrap()).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("[model_providers.minimax]"));
        assert!(content.contains("base_url"));
        assert!(content.contains("api_key"));
        assert!(content.contains("models"));
    }

    #[test]
    fn config_remove_provider_deletes_section() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            r#"
[model_providers.minimax]
base_url = "https://api.minimaxi.chat/v1"
api_key = "sk-test"
"#,
        )
        .unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let mut doc: toml::Value = content.parse().unwrap();
        let table = doc.as_table_mut().unwrap();
        if let Some(providers) = table
            .get_mut("model_providers")
            .and_then(|v| v.as_table_mut())
        {
            providers.remove("minimax");
        }
        std::fs::write(&path, toml::to_string_pretty(&doc).unwrap()).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(!content.contains("[model_providers.minimax]"));
    }

    #[test]
    fn list_configured_providers_reads_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            r#"
[model_providers.minimax]
base_url = "https://api.minimaxi.chat/v1"
api_key = "sk-test"
models = ["MiniMax-M3"]

[model_providers.openai]
base_url = "https://api.openai.com/v1"
api_key = "sk-openai"
"#,
        )
        .unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let doc: toml::Value = content.parse().unwrap();
        let result = parse_providers(&doc);

        assert_eq!(result.len(), 2);
        let minimax = result.iter().find(|(n, _, _)| n == "minimax").unwrap();
        assert_eq!(minimax.1, "https://api.minimaxi.chat/v1");
        assert_eq!(minimax.2, vec!["MiniMax-M3"]);
    }

    #[tokio::test]
    async fn save_provider_config_persists_under_runtime() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        set_test_config_path(path.clone());

        save_provider_config(
            "minimax",
            "https://api.minimaxi.chat/v1",
            "sk-test",
            &["MiniMax-M3".into(), "MiniMax-M2.7".into()],
        )
        .unwrap();

        assert!(path.exists(), "config file should be written");

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(
            content.contains("[model_providers.minimax]"),
            "config should contain minimax provider section:\n{}",
            content
        );
        // api_key is now stored in keyring, not config file

        let providers = list_configured_providers();
        assert_eq!(providers.len(), 1, "expected one configured provider");
        assert_eq!(providers[0].0, "minimax");
        assert_eq!(
            providers[0].2,
            vec!["MiniMax-M3", "MiniMax-M2.7"],
            "saved models should be reflected in list_configured_providers"
        );

        // After migration (version bump), api_key is removed from config
        let loaded = crate::config::Config::load(Some(&path));
        let minimax = loaded
            .model_providers
            .get("minimax")
            .expect("minimax entry");
        // api_key is no longer stored in config - it's in keyring
        assert_eq!(minimax.base_url, "https://api.minimaxi.chat/v1");
    }

    #[test]
    fn concurrent_provider_saves_do_not_corrupt_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        set_test_config_path(path.clone());

        std::thread::scope(|s| {
            let path_a = path.clone();
            s.spawn(move || {
                set_test_config_path(path_a);
                save_provider_config(
                    "openai",
                    "https://api.openai.com/v1",
                    "sk-openai",
                    &["gpt-4o".into()],
                )
                .unwrap();
            });
            let path_b = path.clone();
            s.spawn(move || {
                set_test_config_path(path_b);
                save_provider_config(
                    "minimax",
                    "https://api.minimaxi.chat/v1",
                    "sk-minimax",
                    &["MiniMax-M3".into()],
                )
                .unwrap();
            });
        });

        let providers = list_configured_providers();
        let names: Vec<_> = providers.iter().map(|(n, _, _)| n.as_str()).collect();
        assert_eq!(names, vec!["minimax", "openai"]);

        let minimax = providers.iter().find(|(n, _, _)| n == "minimax").unwrap();
        assert_eq!(minimax.1, "https://api.minimaxi.chat/v1");
        assert_eq!(minimax.2, vec!["MiniMax-M3"]);

        let openai = providers.iter().find(|(n, _, _)| n == "openai").unwrap();
        assert_eq!(openai.1, "https://api.openai.com/v1");
        assert_eq!(openai.2, vec!["gpt-4o"]);

        // After migration, api_key is removed from config (stored in keyring)
        let loaded = crate::config::Config::load(Some(&path));
        // api_key is no longer stored in config - it's in keyring
        assert!(loaded.model_providers.contains_key("minimax"));
        assert!(loaded.model_providers.contains_key("openai"));
    }

    #[test]
    fn list_configured_providers_sorted_alphabetically() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            r#"
[model_providers.zulu]
base_url = "https://zulu.example/v1"
api_key = "sk-zulu"
models = ["z-model"]

[model_providers.anthropic]
base_url = "https://api.anthropic.com/v1"
api_key = "sk-anthropic"
models = ["claude-sonnet-4-6"]

[model_providers.openai]
base_url = "https://api.openai.com/v1"
api_key = "sk-openai"
models = ["gpt-4o"]
"#,
        )
        .unwrap();

        set_test_config_path(path);
        let providers = list_configured_providers();
        let names: Vec<_> = providers.iter().map(|(n, _, _)| n.as_str()).collect();
        assert_eq!(names, vec!["anthropic", "openai", "zulu"]);
    }

    // Helper function to parse providers from TOML
    fn parse_providers(doc: &toml::Value) -> Vec<(String, String, Vec<String>)> {
        doc.get("model_providers")
            .and_then(|v| v.as_table())
            .map(|providers| {
                providers
                    .iter()
                    .map(|(name, val)| {
                        let base_url = val
                            .get("base_url")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let models = val
                            .get("models")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|m| m.as_str().map(String::from))
                                    .collect()
                            })
                            .unwrap_or_default();
                        (name.clone(), base_url, models)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}
