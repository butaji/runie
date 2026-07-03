//! Provider configuration resolver.
//!
//! This module provides credential resolution for LLM providers.
//! Uses the unified `CredentialResolver` from `runie_core::auth`.
//!
//! Priority: 1. Environment variables, 2. dotenvy (.env), 3. OS keyring, 4. Config file

use std::sync::Arc;

use runie_core::proto::ProviderConfig;
use secrecy::SecretString;

/// Resolves provider configuration from multiple sources.
///
/// This is a thin wrapper around `runie_core::auth::CredentialResolver`
/// that adds a `ProviderConfig` fallback for config-level resolution.
#[derive(Clone)]
pub struct ProviderConfigResolver {
    inner: runie_core::auth::CredentialResolver,
    fallback: Option<Arc<dyn ProviderConfig>>,
}

impl std::fmt::Debug for ProviderConfigResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderConfigResolver")
            .field("inner", &self.inner)
            .field("has_fallback", &self.fallback.is_some())
            .finish()
    }
}

impl ProviderConfigResolver {
    /// Create a resolver from a `ProviderConfig` implementation.
    pub fn new(config: Arc<dyn ProviderConfig>) -> Self {
        Self {
            inner: runie_core::auth::CredentialResolver::new(),
            fallback: Some(config),
        }
    }

    /// Create a resolver with only environment variables (no config file).
    pub fn env_only() -> Self {
        Self {
            inner: runie_core::auth::CredentialResolver::new(),
            fallback: None,
        }
    }

    /// Create a resolver with a config but no environment variable capture.
    /// Use this for tests that need isolation from the actual environment.
    #[cfg(test)]
    pub fn with_config<C: runie_core::proto::ProviderConfig + 'static>(config: C) -> Self {
        Self {
            inner: runie_core::auth::CredentialResolver::empty(),
            fallback: Some(Arc::new(config) as Arc<dyn ProviderConfig>),
        }
    }

    /// Create a resolver with an injectable keyring store.
    ///
    /// This allows tests to use `MockKeyringStore` without hitting the OS keyring.
    /// Uses an empty environment to avoid environment variable interference in tests.
    pub fn with_keyring_store(
        config: Arc<dyn ProviderConfig>,
        store: Arc<dyn runie_core::auth::KeyringStore>,
    ) -> Self {
        Self {
            inner: runie_core::auth::CredentialResolver::with_store_empty_env(store),
            fallback: Some(config),
        }
    }

    /// Resolve the API key for a provider, checking environment first.
    ///
    /// Returns a `SecretString` to prevent accidental exposure. Use `.expose_secret()`
    /// only at the HTTP boundary where the key is actually needed.
    pub fn resolve_api_key(&self, provider: &str) -> Option<SecretString> {
        // First try the unified resolver (env, dotenv, keyring)
        if let Some(key) = self.inner.resolve_api_key(provider) {
            return Some(key);
        }
        // Fall back to config
        self.fallback.as_ref()?.resolve_api_key(provider)
    }

    /// Resolve the base URL for a provider, checking environment first.
    pub fn resolve_base_url(&self, provider: &str) -> Option<String> {
        // First try the unified resolver (env, dotenv)
        if let Some(url) = self.inner.resolve_base_url(provider) {
            return Some(url);
        }
        // Fall back to config
        self.fallback.as_ref()?.resolve_base_url(provider)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::ExposeSecret;
    use std::sync::Arc;

    fn ss(s: &str) -> SecretString {
        SecretString::from(s.to_owned())
    }

    struct TestConfig {
        api_key: Option<SecretString>,
        base_url: Option<String>,
    }

    impl std::fmt::Debug for TestConfig {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("TestConfig").finish()
        }
    }

    impl runie_core::proto::ProviderConfig for TestConfig {
        fn resolve_api_key(&self, _provider: &str) -> Option<SecretString> {
            self.api_key.clone()
        }

        fn resolve_base_url(&self, _provider: &str) -> Option<String> {
            self.base_url.clone()
        }
    }

    #[test]
    fn resolve_env_takes_priority() {
        // Set env var BEFORE creating resolver (env is captured at construction)
        std::env::set_var("TESTPROVIDER_API_KEY", "env-key");
        let test_config = TestConfig {
            api_key: Some(ss("config-key")),
            base_url: Some("http://example.com".to_string()),
        };
        let resolver = ProviderConfigResolver::new(
            Arc::new(test_config) as Arc<dyn runie_core::proto::ProviderConfig>
        );

        // Environment variable should override config
        let result = resolver.resolve_api_key("testprovider");
        std::env::remove_var("TESTPROVIDER_API_KEY");

        assert_eq!(
            result.as_ref().map(|s| s.expose_secret().as_str()),
            Some("env-key")
        );
    }

    #[test]
    fn resolve_config_fallback() {
        let test_config = TestConfig {
            api_key: Some(ss("config-key")),
            base_url: Some("http://example.com".to_string()),
        };
        // Use with_config to avoid environment variable interference
        let resolver = ProviderConfigResolver::with_config(test_config);

        // Without env var, should use config
        let result = resolver.resolve_api_key("testprovider");

        assert_eq!(
            result.as_ref().map(|s| s.expose_secret().as_str()),
            Some("config-key")
        );
    }

    #[test]
    fn empty_config_returns_none() {
        let test_config = TestConfig {
            api_key: None,
            base_url: None,
        };
        // Use with_config to avoid environment variable interference
        let resolver = ProviderConfigResolver::with_config(test_config);

        assert!(resolver.resolve_api_key("testprovider").is_none());
        assert_eq!(resolver.resolve_base_url("testprovider"), None);
    }

    #[test]
    fn dotenv_fallback() {
        let test_config = TestConfig {
            api_key: None,
            base_url: None,
        };
        // Use with_config to avoid environment variable interference
        let resolver = ProviderConfigResolver::with_config(test_config);

        // dotenv should be used when config doesn't have the value
        assert!(resolver.resolve_api_key("nonexistent").is_none());
    }

    #[test]
    fn with_keyring_store_uses_mock_keyring() {
        // This test verifies that with_keyring_store uses the injected store
        // instead of the OS keyring.
        use runie_core::auth::MockKeyringStore;

        let test_config = TestConfig {
            api_key: None,
            base_url: Some("http://test.example.com".to_string()),
        };
        let mock_store = Arc::new(MockKeyringStore::new());
        let resolver = ProviderConfigResolver::with_keyring_store(
            Arc::new(test_config) as Arc<dyn runie_core::proto::ProviderConfig>,
            mock_store,
        );

        // Should use config's base_url since no keyring entry exists
        assert_eq!(
            resolver.resolve_base_url("testprovider"),
            Some("http://test.example.com".to_string())
        );
        // Should return None since neither config nor keyring has an API key
        assert!(resolver.resolve_api_key("testprovider").is_none());
    }
}
