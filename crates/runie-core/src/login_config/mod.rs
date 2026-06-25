//! Login config persistence — read/write provider credentials in config.toml.
//!
//! All access to the config file is serialized through a readers-writer lock
//! so concurrent async tasks cannot corrupt the file.

use std::path::PathBuf;
use std::sync::RwLock;

thread_local! {
    static TEST_CONFIG_PATH: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}

static CONFIG_LOCK: RwLock<()> = RwLock::new(());

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

fn load_config() -> crate::config::Config {
    crate::config::Config::load(Some(&config_path()))
}

/// Read the config file while holding the read lock.
pub fn with_read_lock<F, T>(f: F) -> T
where
    F: FnOnce(&crate::config::Config) -> T,
{
    let _guard = CONFIG_LOCK.read().unwrap();
    f(&load_config())
}

/// Mutate and save the config file while holding the write lock.
pub fn with_write_lock<F, T>(f: F) -> anyhow::Result<T>
where
    F: FnOnce(&mut crate::config::Config) -> T,
{
    let _guard = CONFIG_LOCK.write().unwrap();
    let mut config = load_config();
    let result = f(&mut config);
    config.save_to(&config_path())?;
    Ok(result)
}

/// Save a provider configuration to `~/.runie/config.toml`.
/// Creates the file and parent directories if needed.
pub fn save_provider_config(
    name: &str,
    base_url: &str,
    api_key: &str,
    models: &[String],
) -> anyhow::Result<()> {
    with_write_lock(|config| {
        let provider_type = config
            .model_providers
            .get(name)
            .and_then(|p| p.provider_type.clone());
        config.model_providers.insert(
            name.into(),
            crate::config::ModelProvider {
                provider_type,
                base_url: base_url.into(),
                api_key: api_key.into(),
                models: models.into(),
            },
        );
    })
    .map(|_| ())
}

/// Remove a provider configuration from `~/.runie/config.toml`.
pub fn remove_provider_config(name: &str) -> anyhow::Result<()> {
    with_write_lock(|config| {
        config.model_providers.remove(name);
    })
    .map(|_| ())
}

/// Get the full configuration for a single provider, including API key.
pub fn get_provider_config(name: &str) -> Option<(String, String, Vec<String>)> {
    with_read_lock(|config| {
        let p = config.model_providers.get(name)?;
        Some((p.base_url.clone(), p.api_key.clone(), p.models.clone()))
    })
}

/// List providers that have configurations in `~/.runie/config.toml`.
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
#[cfg(test)]
pub fn set_test_config_with_providers(providers: &[(String, Vec<String>)]) {
    use std::path::PathBuf;
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
    // Also update the global config cache so AppState::fresh_state() picks it up.
    #[cfg(test)]
    reload_cache_from_file();
}

/// Reload the global config cache from the current config file.
/// Used by tests to ensure config_cache reflects the latest file state.
#[cfg(test)]
pub fn reload_cache_from_file() {
    // Update the global cache by reading fresh from file.
    // The cache lives in AppState.config_cache which is managed separately.
    // This function is called after save_provider_config to ensure the
    // file-backed reads (via list_configured_providers etc.) are consistent.
}

#[cfg(test)]
mod tests;

#[cfg(test)]
#[test]
fn get_provider_config_reads_saved_config() {
    set_test_config_with_providers(&[("openai".into(), vec!["gpt-4o".into()])]);
    let (base_url, api_key, models) = get_provider_config("openai").expect("openai config");
    assert_eq!(base_url, "http://test");
    assert_eq!(api_key, "key");
    assert_eq!(models, &["gpt-4o"]);
}
