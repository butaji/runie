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
/// 4. Legacy `auth.json` file (CI/headless fallback)
/// 5. Config file
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
    /// Tokens loaded from the legacy `auth.json` file (provider -> token),
    /// keyed by lowercased provider name. Used as a fallback when the OS
    /// keyring is unavailable or unreadable (e.g. CI/headless, or a Keychain
    /// access failure), matching `AuthStorage`'s keyring-or-file behaviour.
    file_tokens: HashMap<String, SecretString>,
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
            .field("file_tokens", &self.file_tokens)
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
            file_tokens: Self::load_auth_file(),
            entries: HashMap::new(),
            store: Self::default_store(),
        }
    }

    /// Create a resolver with an empty environment (for testing).
    pub fn empty() -> Self {
        Self {
            env: HashMap::new(),
            dotenv: HashMap::new(),
            file_tokens: HashMap::new(),
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
            file_tokens: Self::load_auth_file(),
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
            file_tokens: HashMap::new(),
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

    /// Load tokens from the legacy `auth.json` file
    /// (`dirs::data_dir()/runie/auth.json`), the same file `AuthStorage` uses as
    /// its keyring fallback.
    ///
    /// Returns an empty map when the file is missing or unreadable. Provider
    /// keys are lowercased so lookup is case-insensitive, matching config
    /// resolution. This keeps credentials reachable when the OS keyring is
    /// unavailable or unreadable (CI/headless, or a macOS Keychain access
    /// failure) instead of failing a turn with `MissingApiKey`.
    fn load_auth_file() -> HashMap<String, SecretString> {
        // `RUNIE_AUTH_FILE` overrides the path, primarily so tests (and CI) can
        // stay hermetic instead of reading the developer's real credentials.
        // Falls back to the standard `dirs::data_dir()/runie/auth.json`.
        let path = std::env::var_os("RUNIE_AUTH_FILE")
            .map(std::path::PathBuf::from)
            .or_else(|| dirs::data_dir().map(|d| d.join("runie").join("auth.json")));
        let Some(path) = path else {
            return HashMap::new();
        };
        Self::load_auth_file_from(&path)
    }

    fn load_auth_file_from(path: &std::path::Path) -> HashMap<String, SecretString> {
        let mut out = HashMap::new();
        let Ok(json) = std::fs::read_to_string(path) else {
            return out;
        };
        let raw: serde_json::Value = serde_json::from_str(&json).unwrap_or(serde_json::json!({}));
        if let Some(obj) = raw.as_object() {
            for (provider, val) in obj {
                let Some(token) = val.get("token").and_then(|v| v.as_str()) else {
                    continue;
                };
                if token.is_empty() {
                    continue;
                }
                out.insert(
                    provider.to_lowercase(),
                    SecretString::from(token.to_owned()),
                );
            }
        }
        out
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
        self.resolve_api_key_with_env_vars(provider, &[])
    }

    /// Resolve the API key, preferring the given env var names before falling
    /// back to the default `{PROVIDER}_API_KEY` form. This lets providers with
    /// hyphens in their key (e.g. `kimi-code`) declare a clean env var such as
    /// `KIMI_API_KEY` while still accepting the derived form for backwards
    /// compatibility.
    pub fn resolve_api_key_with_env_vars(
        &self,
        provider: &str,
        preferred_env_vars: &[String],
    ) -> Option<SecretString> {
        let default_env_key = format!("{}_API_KEY", provider.to_uppercase());
        let mut env_keys: Vec<String> = preferred_env_vars.to_vec();
        if !env_keys.contains(&default_env_key) {
            env_keys.push(default_env_key);
        }

        // 1. Environment variables (preferred first, then default)
        for env_key in &env_keys {
            if let Some(val) = self.env.get(env_key) {
                if !val.expose_secret().is_empty() {
                    return Some(val.clone());
                }
            }
        }

        // 2. .env file
        for env_key in &env_keys {
            if let Some(val) = self.dotenv.get(env_key) {
                if !val.expose_secret().is_empty() {
                    return Some(val.clone());
                }
            }
        }

        // 3. Keyring (via injectable store) - already returns SecretString
        if let Ok(Some(token)) = self.store.get(provider) {
            return Some(token);
        }

        // 4. Legacy auth.json file (CI/headless / keyring-unavailable fallback)
        if let Some(token) = self.file_tokens.get(&provider.to_lowercase()) {
            if !token.expose_secret().is_empty() {
                return Some(token.clone());
            }
        }

        // 5. Config file
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

    #[test]
    fn load_auth_file_from_parses_and_lowercases() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        std::fs::write(
            &path,
            r#"{"MiniMax":{"token":"mm-file-token","expires_at":0},"Empty":{"token":""}}"#,
        )
        .unwrap();

        let tokens = CredentialResolver::load_auth_file_from(&path);
        assert_secret_eq(tokens.get("minimax"), "mm-file-token");
        assert!(
            !tokens.contains_key("empty"),
            "empty tokens must be skipped"
        );
    }

    #[test]
    fn load_auth_file_from_missing_returns_empty() {
        let path = std::path::Path::new("/definitely/not/here/runie_auth_missing.json");
        assert!(CredentialResolver::load_auth_file_from(path).is_empty());
    }

    #[test]
    fn resolver_falls_back_to_auth_file() {
        let mock: Arc<dyn KeyringStore> = Arc::new(MockKeyringStore::new());
        let mut resolver = CredentialResolver::with_store_empty_env(mock);
        resolver
            .file_tokens
            .insert("minimax".to_owned(), ss("file-token"));

        assert_secret_eq(resolver.resolve_api_key("minimax").as_ref(), "file-token");
    }

    #[test]
    fn resolver_prefers_keyring_over_auth_file() {
        let mock: Arc<dyn KeyringStore> = Arc::new(MockKeyringStore::new());
        mock.set("minimax", "keyring-token").unwrap();

        let mut resolver = CredentialResolver::with_store_empty_env(Arc::clone(&mock));
        resolver
            .file_tokens
            .insert("minimax".to_owned(), ss("file-token"));

        // Keyring should win over the auth.json file
        assert_secret_eq(
            resolver.resolve_api_key("minimax").as_ref(),
            "keyring-token",
        );
    }

    #[test]
    fn resolver_prefers_auth_file_over_config() {
        let mock: Arc<dyn KeyringStore> = Arc::new(MockKeyringStore::new());
        let mut resolver = CredentialResolver::with_store_empty_env(mock);
        resolver
            .file_tokens
            .insert("minimax".to_owned(), ss("file-token"));
        resolver.set_config("minimax", Some(ss("config-token")), None);

        // auth.json file should win over config
        assert_secret_eq(resolver.resolve_api_key("minimax").as_ref(), "file-token");
    }
}
