//! Unified credential resolver for API keys and base URLs.

use std::collections::HashMap;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Unified Credential Resolver
// ---------------------------------------------------------------------------

/// Unified credential resolver implementing consistent priority:
/// 1. Environment variables
/// 2. .env file (via dotenvy)
/// 3. OS keyring
/// 4. Config file
///
/// The keyring backend is injectable via `Arc<dyn KeyringStore>`, enabling
/// tests to use `MockKeyringStore` without OS keychain access.
#[derive(Clone)]
pub struct CredentialResolver {
    /// Environment variables captured at construction.
    env: HashMap<String, String>,
    /// Variables loaded from .env file via dotenvy.
    dotenv: HashMap<String, String>,
    /// Provider config entries (provider name -> (api_key, base_url)).
    entries: HashMap<String, (Option<String>, Option<String>)>,
    /// Keyring storage backend (injectable for tests).
    store: Arc<dyn super::KeyringStore>,
}

impl std::fmt::Debug for CredentialResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CredentialResolver")
            .field("env", &self.env)
            .field("dotenv", &self.dotenv)
            .field("entries", &self.entries)
            .finish()
    }
}

impl Default for CredentialResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl CredentialResolver {
    /// Create a new resolver using the OS keyring.
    pub fn new() -> Self {
        let env: HashMap<String, String> = std::env::vars().collect();
        let dotenv = Self::load_dotenv();
        Self {
            env,
            dotenv,
            entries: HashMap::new(),
            store: Arc::new(super::OsKeyringStore::new()),
        }
    }

    /// Create a resolver with an empty environment (for testing).
    pub fn empty() -> Self {
        Self {
            env: HashMap::new(),
            dotenv: HashMap::new(),
            entries: HashMap::new(),
            store: Arc::new(super::OsKeyringStore::new()),
        }
    }

    /// Create a resolver with an injectable keyring store (for tests).
    pub fn with_store(store: Arc<dyn super::KeyringStore>) -> Self {
        let env: HashMap<String, String> = std::env::vars().collect();
        let dotenv = Self::load_dotenv();
        Self {
            env,
            dotenv,
            entries: HashMap::new(),
            store,
        }
    }

    /// Load .env file using dotenvy, returning only API-related variables.
    ///
    /// Uses `dotenvy::from_filename_iter` to read the .env file directly into
    /// a HashMap without mutating the process environment.
    fn load_dotenv() -> HashMap<String, String> {
        match dotenvy::from_filename_iter(".env") {
            Ok(iter) => iter
                .filter_map(|result| result.ok())
                .filter(|(key, _)| key.ends_with("_API_KEY") || key.ends_with("_BASE_URL"))
                .collect(),
            Err(_) => HashMap::new(),
        }
    }

    /// Set a config entry for a provider.
    pub fn set_config(
        &mut self,
        provider: &str,
        api_key: Option<String>,
        base_url: Option<String>,
    ) {
        self.entries.insert(provider.to_lowercase(), (api_key, base_url));
    }

    /// Resolve the API key for a provider using the standard priority chain.
    pub fn resolve_api_key(&self, provider: &str) -> Option<String> {
        let env_key = format!("{}_API_KEY", provider.to_uppercase());

        // 1. Environment variable
        if let Some(val) = self.env.get(&env_key) {
            if !val.is_empty() {
                return Some(val.clone());
            }
        }

        // 2. .env file
        if let Some(val) = self.dotenv.get(&env_key) {
            if !val.is_empty() {
                return Some(val.clone());
            }
        }

        // 3. Keyring (via injectable store)
        if let Ok(Some(token)) = self.store.get(provider) {
            return Some(token);
        }

        // 4. Config file
        self.entries
            .get(&provider.to_lowercase())
            .and_then(|(api_key, _)| api_key.clone())
            .filter(|s| !s.is_empty())
    }

    /// Resolve the base URL for a provider using the standard priority chain.
    pub fn resolve_base_url(&self, provider: &str) -> Option<String> {
        let env_key = format!("{}_BASE_URL", provider.to_uppercase());

        // 1. Environment variable
        if let Some(val) = self.env.get(&env_key) {
            if !val.is_empty() {
                return Some(val.clone());
            }
        }

        // 2. .env file
        if let Some(val) = self.dotenv.get(&env_key) {
            if !val.is_empty() {
                return Some(val.clone());
            }
        }

        // 3. Config file (no keyring for base URLs)
        self.entries
            .get(&provider.to_lowercase())
            .and_then(|(_, base_url)| base_url.clone())
            .filter(|s| !s.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use super::super::{KeyringStore, MockKeyringStore};
    use super::*;

    #[test]
    fn resolver_priority_env_over_keyring() {
        let mut resolver = CredentialResolver::empty();
        resolver.env.insert("TESTPROVIDER_API_KEY".to_owned(), "env-key".to_owned());

        let result = resolver.resolve_api_key("testprovider");
        assert_eq!(result, Some("env-key".to_owned()));
    }

    #[test]
    fn resolver_fallback_to_config() {
        let mut resolver = CredentialResolver::empty();
        resolver.set_config(
            "testprovider",
            Some("config-key".to_owned()),
            Some("http://config".to_owned()),
        );

        assert_eq!(
            resolver.resolve_api_key("testprovider"),
            Some("config-key".to_owned())
        );
        assert_eq!(
            resolver.resolve_base_url("testprovider"),
            Some("http://config".to_owned())
        );
    }

    #[test]
    fn resolver_prefers_env_over_dotenv() {
        let mut resolver = CredentialResolver::empty();
        resolver
            .env
            .insert("TEST_API_KEY".to_owned(), "env-key".to_owned());
        resolver.dotenv.insert("TEST_API_KEY".to_owned(), "dotenv-key".to_owned());

        assert_eq!(
            resolver.resolve_api_key("test"),
            Some("env-key".to_owned())
        );
    }

    #[test]
    fn resolver_prefers_dotenv_over_config() {
        let mut resolver = CredentialResolver::empty();
        resolver.dotenv.insert("TEST_API_KEY".to_owned(), "dotenv-key".to_owned());
        resolver.set_config("test", Some("config-key".to_owned()), None);

        assert_eq!(
            resolver.resolve_api_key("test"),
            Some("dotenv-key".to_owned())
        );
    }

    #[test]
    fn load_dotenv_does_not_mutate_process_env() {
        let before = std::env::var("TEST_DOTENV_SANITY_KEY");
        let _resolver = CredentialResolver::new();
        let after = std::env::var("TEST_DOTENV_SANITY_KEY");
        assert_eq!(before, after);
    }

    #[test]
    fn resolver_uses_injected_mock_store() {
        let mock: Arc<dyn KeyringStore> = Arc::new(MockKeyringStore::new());
        let resolver = CredentialResolver::with_store(Arc::clone(&mock));

        // Pre-load the mock store
        mock.set("injected", "mock-token").unwrap();

        assert_eq!(
            resolver.resolve_api_key("injected"),
            Some("mock-token".to_owned())
        );
    }

    #[test]
    fn resolver_mock_store_gets_none_when_empty() {
        let mock: Arc<dyn KeyringStore> = Arc::new(MockKeyringStore::new());
        let resolver = CredentialResolver::with_store(mock);

        // Provider not in mock store, not in env, not in dotenv, not in config
        assert_eq!(resolver.resolve_api_key("missing"), None);
    }

    #[test]
    fn resolver_mock_store_priority_env_over_mock() {
        let mock: Arc<dyn KeyringStore> = Arc::new(MockKeyringStore::new());
        mock.set("priority_test", "mock-token").unwrap();

        let mut resolver = CredentialResolver::with_store(Arc::clone(&mock));
        resolver.env.insert("PRIORITY_TEST_API_KEY".to_owned(), "env-token".to_owned());

        // Env should win over mock store
        assert_eq!(
            resolver.resolve_api_key("priority_test"),
            Some("env-token".to_owned())
        );
    }

    #[test]
    fn resolver_mock_store_priority_dotenv_over_mock() {
        let mock: Arc<dyn KeyringStore> = Arc::new(MockKeyringStore::new());
        mock.set("dotenv_test", "mock-token").unwrap();

        let mut resolver = CredentialResolver::with_store(Arc::clone(&mock));
        resolver.dotenv.insert("DOTENV_TEST_API_KEY".to_owned(), "dotenv-token".to_owned());

        // dotenv should win over mock store
        assert_eq!(
            resolver.resolve_api_key("dotenv_test"),
            Some("dotenv-token".to_owned())
        );
    }

    #[test]
    fn resolver_mock_store_priority_mock_over_config() {
        let mock: Arc<dyn KeyringStore> = Arc::new(MockKeyringStore::new());
        mock.set("config_test", "mock-token").unwrap();

        let mut resolver = CredentialResolver::with_store(Arc::clone(&mock));
        resolver.set_config("config_test", Some("config-token".to_owned()), None);

        // mock store should win over config
        assert_eq!(
            resolver.resolve_api_key("config_test"),
            Some("mock-token".to_owned())
        );
    }
}
