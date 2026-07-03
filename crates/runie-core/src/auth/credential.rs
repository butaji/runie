//! Unified credential resolver for API keys and base URLs.

use std::collections::HashMap;
use std::sync::Arc;

use secrecy::{ExposeSecret, SecretString};

// ---------------------------------------------------------------------------
// Unified Credential Resolver
// ---------------------------------------------------------------------------

/// Unified credential resolver implementing consistent priority:
/// 1. Environment variables
/// 2. .env file (via dotenvy)
/// 3. OS keyring
/// 4. Config file
///
/// API keys are stored as `SecretString` throughout the resolver to prevent
/// accidental exposure in logs or error messages. The secret value is only
/// revealed at the HTTP boundary via `ExposeSecret`.
///
/// The keyring backend is injectable via `Arc<dyn KeyringStore>`, enabling
/// tests to use `MockKeyringStore` without OS keychain access.
#[derive(Clone)]
pub struct CredentialResolver {
    /// Environment variables captured at construction.
    env: HashMap<String, SecretString>,
    /// Variables loaded from .env file via dotenvy.
    dotenv: HashMap<String, SecretString>,
    /// Provider config entries (provider name -> (api_key, base_url)).
    entries: HashMap<String, (Option<SecretString>, Option<String>)>,
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
    /// Falls back to `MockKeyringStore` when the `keyring` feature is disabled.
    pub fn new() -> Self {
        let env: HashMap<String, SecretString> = std::env::vars()
            .map(|(k, v)| (k, SecretString::from(v)))
            .collect();
        let dotenv = Self::load_dotenv();
        Self {
            env,
            dotenv,
            entries: HashMap::new(),
            store: Self::default_store(),
        }
    }

    /// Create a resolver with an empty environment (for testing).
    pub fn empty() -> Self {
        Self {
            env: HashMap::new(),
            dotenv: HashMap::new(),
            entries: HashMap::new(),
            store: Self::default_store(),
        }
    }

    /// Returns the default keyring store, falling back to `MockKeyringStore`
    /// when the `keyring` feature is not enabled.
    fn default_store() -> Arc<dyn super::KeyringStore> {
        #[cfg(feature = "keyring")]
        {
            Arc::new(super::OsKeyringStore::new())
        }
        #[cfg(not(feature = "keyring"))]
        {
            Arc::new(super::MockKeyringStore::new())
        }
    }

    /// Create a resolver with an injectable keyring store (for tests).
    pub fn with_store(store: Arc<dyn super::KeyringStore>) -> Self {
        let env: HashMap<String, SecretString> = std::env::vars()
            .map(|(k, v)| (k, SecretString::from(v)))
            .collect();
        let dotenv = Self::load_dotenv();
        Self {
            env,
            dotenv,
            entries: HashMap::new(),
            store,
        }
    }

    /// Create a resolver with an injectable keyring store and empty environment.
    ///
    /// This is useful for tests that need isolation from both the environment
    /// and the OS keyring.
    pub fn with_store_empty_env(store: Arc<dyn super::KeyringStore>) -> Self {
        Self {
            env: HashMap::new(),
            dotenv: HashMap::new(),
            entries: HashMap::new(),
            store,
        }
    }

    /// Load .env file using dotenvy, returning only API-related variables.
    ///
    /// Uses `dotenvy::from_filename_iter` to read the .env file directly into
    /// a HashMap without mutating the process environment.
    fn load_dotenv() -> HashMap<String, SecretString> {
        match dotenvy::from_filename_iter(".env") {
            Ok(iter) => iter
                .filter_map(|result| result.ok())
                .filter(|(key, _)| key.ends_with("_API_KEY") || key.ends_with("_BASE_URL"))
                .map(|(k, v)| (k, SecretString::from(v)))
                .collect(),
            Err(_) => HashMap::new(),
        }
    }

    /// Set a config entry for a provider.
    pub fn set_config(
        &mut self,
        provider: &str,
        api_key: Option<SecretString>,
        base_url: Option<String>,
    ) {
        self.entries
            .insert(provider.to_lowercase(), (api_key, base_url));
    }

    /// Resolve the API key for a provider using the standard priority chain.
    ///
    /// Returns a `SecretString` to prevent accidental exposure. Use `.expose_secret()`
    /// only at the HTTP boundary where the key is actually needed.
    pub fn resolve_api_key(&self, provider: &str) -> Option<SecretString> {
        let env_key = format!("{}_API_KEY", provider.to_uppercase());

        // 1. Environment variable
        if let Some(val) = self.env.get(&env_key) {
            if !val.expose_secret().is_empty() {
                return Some(val.clone());
            }
        }

        // 2. .env file
        if let Some(val) = self.dotenv.get(&env_key) {
            if !val.expose_secret().is_empty() {
                return Some(val.clone());
            }
        }

        // 3. Keyring (via injectable store) - already returns SecretString
        if let Ok(Some(token)) = self.store.get(provider) {
            return Some(token);
        }

        // 4. Config file
        self.entries
            .get(&provider.to_lowercase())
            .and_then(|(api_key, _)| api_key.clone())
            .filter(|s| !s.expose_secret().is_empty())
    }

    /// Resolve the base URL for a provider using the standard priority chain.
    pub fn resolve_base_url(&self, provider: &str) -> Option<String> {
        let env_key = format!("{}_BASE_URL", provider.to_uppercase());

        // 1. Environment variable (now stored as SecretString, expose for base_url)
        if let Some(val) = self.env.get(&env_key) {
            let s = val.expose_secret();
            if !s.is_empty() {
                return Some(s.clone());
            }
        }

        // 2. .env file (now stored as SecretString, expose for base_url)
        if let Some(val) = self.dotenv.get(&env_key) {
            let s = val.expose_secret();
            if !s.is_empty() {
                return Some(s.clone());
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
    use super::super::{KeyringStore, MockKeyringStore};
    use super::*;
    use secrecy::{ExposeSecret, SecretString};
    use std::sync::Arc;

    fn ss(s: &str) -> SecretString {
        SecretString::from(s.to_owned())
    }

    /// Helper to compare Option<SecretString> by exposing the secret.
    fn assert_secret_eq(result: Option<&SecretString>, expected: &str) {
        match result {
            Some(s) => assert_eq!(s.expose_secret(), expected),
            None => panic!("expected Some({:?}), got None", expected),
        }
    }

    #[test]
    fn resolver_priority_env_over_keyring() {
        let mut resolver = CredentialResolver::empty();
        resolver
            .env
            .insert("TESTPROVIDER_API_KEY".to_owned(), ss("env-key"));

        let result = resolver.resolve_api_key("testprovider");
        assert_secret_eq(result.as_ref(), "env-key");
    }

    #[test]
    fn resolver_fallback_to_config() {
        let mut resolver = CredentialResolver::empty();
        resolver.set_config(
            "testprovider",
            Some(ss("config-key")),
            Some("http://config".to_owned()),
        );

        assert_secret_eq(
            resolver.resolve_api_key("testprovider").as_ref(),
            "config-key",
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
            .insert("TEST_API_KEY".to_owned(), ss("env-key"));
        resolver
            .dotenv
            .insert("TEST_API_KEY".to_owned(), ss("dotenv-key"));

        assert_secret_eq(resolver.resolve_api_key("test").as_ref(), "env-key");
    }

    #[test]
    fn resolver_prefers_dotenv_over_config() {
        let mut resolver = CredentialResolver::empty();
        resolver
            .dotenv
            .insert("TEST_API_KEY".to_owned(), ss("dotenv-key"));
        resolver.set_config("test", Some(ss("config-key")), None);

        assert_secret_eq(resolver.resolve_api_key("test").as_ref(), "dotenv-key");
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

        assert_secret_eq(resolver.resolve_api_key("injected").as_ref(), "mock-token");
    }

    #[test]
    fn resolver_mock_store_gets_none_when_empty() {
        let mock: Arc<dyn KeyringStore> = Arc::new(MockKeyringStore::new());
        let resolver = CredentialResolver::with_store(mock);

        // Provider not in mock store, not in env, not in dotenv, not in config
        assert!(resolver.resolve_api_key("missing").is_none());
    }

    #[test]
    fn resolver_mock_store_priority_env_over_mock() {
        let mock: Arc<dyn KeyringStore> = Arc::new(MockKeyringStore::new());
        mock.set("priority_test", "mock-token").unwrap();

        let mut resolver = CredentialResolver::with_store(Arc::clone(&mock));
        resolver
            .env
            .insert("PRIORITY_TEST_API_KEY".to_owned(), ss("env-token"));

        // Env should win over mock store
        assert_secret_eq(
            resolver.resolve_api_key("priority_test").as_ref(),
            "env-token",
        );
    }

    #[test]
    fn resolver_mock_store_priority_dotenv_over_mock() {
        let mock: Arc<dyn KeyringStore> = Arc::new(MockKeyringStore::new());
        mock.set("dotenv_test", "mock-token").unwrap();

        let mut resolver = CredentialResolver::with_store(Arc::clone(&mock));
        resolver
            .dotenv
            .insert("DOTENV_TEST_API_KEY".to_owned(), ss("dotenv-token"));

        // dotenv should win over mock store
        assert_secret_eq(
            resolver.resolve_api_key("dotenv_test").as_ref(),
            "dotenv-token",
        );
    }

    #[test]
    fn resolver_mock_store_priority_mock_over_config() {
        let mock: Arc<dyn KeyringStore> = Arc::new(MockKeyringStore::new());
        mock.set("config_test", "mock-token").unwrap();

        let mut resolver = CredentialResolver::with_store(Arc::clone(&mock));
        resolver.set_config("config_test", Some(ss("config-token")), None);

        // mock store should win over config
        assert_secret_eq(
            resolver.resolve_api_key("config_test").as_ref(),
            "mock-token",
        );
    }
}
