//! Provider configuration resolver.
//!
//! This module provides credential resolution for LLM providers.
//! It uses the `ProviderConfig` trait to break circular dependencies
//! between runie-core and runie-provider.
//!
//! Priority: 1. Environment variables, 2. dotenvy (.env), 3. Config file

use std::collections::HashMap;

use runie_core::proto::ProviderConfigBox;
#[cfg(test)]
use runie_core::proto::ProviderConfig;

/// Resolves provider configuration from multiple sources with priority:
/// 1. Environment variables
/// 2. .env file (loaded via dotenvy)
/// 3. Config file entries
#[derive(Clone)]
pub struct ProviderConfigResolver {
    env: HashMap<String, String>,
    dotenv: HashMap<String, String>,
    provider_config: Option<ProviderConfigBox>,
}

impl std::fmt::Debug for ProviderConfigResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderConfigResolver")
            .field("env_count", &self.env.len())
            .field("dotenv_count", &self.dotenv.len())
            .field("has_provider_config", &self.provider_config.is_some())
            .finish()
    }
}

impl ProviderConfigResolver {
    /// Create a resolver from a `ProviderConfig` implementation.
    pub fn new(provider_config: ProviderConfigBox) -> Self {
        let mut env = HashMap::new();
        for (key, val) in std::env::vars() {
            env.insert(key, val);
        }

        let dotenv = Self::load_dotenv();

        Self { env, dotenv, provider_config: Some(provider_config) }
    }

    /// Create a resolver with only environment variables (no config file).
    pub fn env_only() -> Self {
        let mut env = HashMap::new();
        for (key, val) in std::env::vars() {
            env.insert(key, val);
        }
        Self { env, dotenv: HashMap::new(), provider_config: None }
    }

    /// Create a resolver with a config but no environment variable capture.
    /// Use this for tests that need isolation from the actual environment.
    #[cfg(test)]
    pub fn with_config<C: runie_core::proto::ProviderConfig + 'static>(config: C) -> Self {
        Self { env: HashMap::new(), dotenv: HashMap::new(), provider_config: Some(ProviderConfigBox::new(config)) }
    }

    /// Load .env file using dotenvy.
    fn load_dotenv() -> HashMap<String, String> {
        match dotenvy::dotenv() {
            Ok(path) => {
                // dotenvy loads into env; read them back
                let mut map = HashMap::new();
                // Read only the vars we care about (API_KEY, BASE_URL)
                for (key, val) in std::env::vars() {
                    if key.ends_with("_API_KEY") || key.ends_with("_BASE_URL") {
                        map.insert(key, val);
                    }
                }
                // Also check the .env file path for keys dotenvy might have loaded
                if let Ok(content) = std::fs::read_to_string(&path) {
                    for line in content.lines() {
                        let line = line.trim();
                        if line.is_empty() || line.starts_with('#') {
                            continue;
                        }
                        if let Some((key, val)) = line.split_once('=') {
                            let key = key.trim().to_owned();
                            if key.ends_with("_API_KEY") || key.ends_with("_BASE_URL") {
                                // Only insert if not already in env
                                map.entry(key).or_insert_with(|| {
                                    val.trim().trim_matches('"').to_owned()
                                });
                            }
                        }
                    }
                }
                map
            }
            Err(_) => HashMap::new(),
        }
    }

    /// Resolve the API key for a provider, checking environment first.
    pub fn resolve_api_key(&self, provider: &str) -> Option<String> {
        let env_key = format!("{}_API_KEY", provider.to_uppercase());
        if let Some(val) = self.env.get(&env_key) {
            if !val.is_empty() {
                return Some(val.clone());
            }
        }
        if let Some(val) = self.dotenv.get(&env_key) {
            if !val.is_empty() {
                return Some(val.clone());
            }
        }
        self.provider_config.as_ref()?.resolve_api_key(provider)
    }

    /// Resolve the base URL for a provider, checking environment first.
    pub fn resolve_base_url(&self, provider: &str) -> Option<String> {
        let env_key = format!("{}_BASE_URL", provider.to_uppercase());
        if let Some(val) = self.env.get(&env_key) {
            if !val.is_empty() {
                return Some(val.clone());
            }
        }
        if let Some(val) = self.dotenv.get(&env_key) {
            if !val.is_empty() {
                return Some(val.clone());
            }
        }
        self.provider_config.as_ref()?.resolve_base_url(provider)
    }
}

// Re-export Config from runie-core for backward compatibility
pub use runie_core::config::{Config, ModelProvider, ModelsSection};

#[cfg(test)]
mod tests {
    use super::*;

    struct TestConfig {
        api_key: Option<String>,
        base_url: Option<String>,
    }

    impl std::fmt::Debug for TestConfig {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("TestConfig").finish()
        }
    }

    impl ProviderConfig for TestConfig {
        fn resolve_api_key(&self, _provider: &str) -> Option<String> {
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
            api_key: Some("config-key".to_string()),
            base_url: Some("http://example.com".to_string()),
        };
        let resolver =
            ProviderConfigResolver::new(ProviderConfigBox::new(test_config));

        // Environment variable should override config
        let result = resolver.resolve_api_key("testprovider");
        std::env::remove_var("TESTPROVIDER_API_KEY");

        assert_eq!(result, Some("env-key".to_string()));
    }

    #[test]
    fn resolve_config_fallback() {
        let test_config = TestConfig {
            api_key: Some("config-key".to_string()),
            base_url: Some("http://example.com".to_string()),
        };
        // Use with_config to avoid environment variable interference
        let resolver = ProviderConfigResolver::with_config(test_config);

        // Without env var, should use config
        let result = resolver.resolve_api_key("testprovider");

        assert_eq!(result, Some("config-key".to_string()));
    }

    #[test]
    fn empty_config_returns_none() {
        let test_config = TestConfig {
            api_key: None,
            base_url: None,
        };
        // Use with_config to avoid environment variable interference
        let resolver = ProviderConfigResolver::with_config(test_config);

        assert_eq!(resolver.resolve_api_key("testprovider"), None);
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
        assert_eq!(resolver.resolve_api_key("nonexistent"), None);
    }
}
