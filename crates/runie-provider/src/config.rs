//! Global TOML config (~/.runie/config.toml)
//!
//! This module re-exports from the canonical config types in runie-core.

pub use runie_core::config::{Config, ModelProvider, ModelsSection};

/// Resolves provider configuration from multiple sources with priority:
/// 1. Environment variables
/// 2. .env file in current working directory
/// 3. config.toml model_providers section
#[derive(Debug, Clone, Default)]
pub struct ProviderConfigResolver {
    env: std::collections::HashMap<String, String>,
    dotenv: std::collections::HashMap<String, String>,
    config_file: std::collections::HashMap<String, ModelProvider>,
}

impl ProviderConfigResolver {
    pub fn from_config(config: &Config) -> Self {
        let mut env = std::collections::HashMap::new();
        for (key, val) in std::env::vars() {
            env.insert(key, val);
        }

        let dotenv = Self::load_dotenv();

        Self {
            env,
            dotenv,
            config_file: config.model_providers.clone(),
        }
    }

    fn load_dotenv() -> std::collections::HashMap<String, String> {
        let path = std::env::current_dir().unwrap_or_default().join(".env");
        if !path.exists() {
            return std::collections::HashMap::new();
        }
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return std::collections::HashMap::new(),
        };
        let mut map = std::collections::HashMap::new();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, val)) = line.split_once('=') {
                map.insert(
                    key.trim().to_string(),
                    val.trim().trim_matches('"').to_string(),
                );
            }
        }
        map
    }

    pub fn resolve_api_key(&self, provider: &str) -> Option<String> {
        let env_key = format!("{}_API_KEY", provider.to_uppercase());
        if let Some(val) = self.env.get(&env_key) {
            return Some(val.clone());
        }
        if let Some(val) = self.dotenv.get(&env_key) {
            return Some(val.clone());
        }
        self.config_file.get(provider).map(|p| p.api_key.clone())
    }

    pub fn resolve_base_url(&self, provider: &str) -> Option<String> {
        let env_key = format!("{}_BASE_URL", provider.to_uppercase());
        if let Some(val) = self.env.get(&env_key) {
            return Some(val.clone());
        }
        if let Some(val) = self.dotenv.get(&env_key) {
            return Some(val.clone());
        }
        self.config_file.get(provider).map(|p| p.base_url.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_env_takes_priority() {
        let config = Config::default();
        let mut resolver = ProviderConfigResolver::from_config(&config);
        resolver
            .env
            .insert("TESTPROVIDER_API_KEY".to_string(), "env-key".to_string());
        resolver.config_file.insert(
            "testprovider".to_string(),
            ModelProvider {
                provider_type: None,
                base_url: "http://example.com".to_string(),
                api_key: "config-key".to_string(),
            },
        );

        assert_eq!(
            resolver.resolve_api_key("testprovider"),
            Some("env-key".to_string())
        );
    }

    #[test]
    fn resolve_dotenv_fallback() {
        let config = Config::default();
        let mut resolver = ProviderConfigResolver::from_config(&config);
        resolver
            .dotenv
            .insert("TESTPROVIDER_API_KEY".to_string(), "dotenv-key".to_string());

        assert_eq!(
            resolver.resolve_api_key("testprovider"),
            Some("dotenv-key".to_string())
        );
    }

    #[test]
    fn resolve_config_fallback() {
        let mut config = Config::default();
        config.model_providers.insert(
            "testprovider".to_string(),
            ModelProvider {
                provider_type: None,
                base_url: "http://example.com".to_string(),
                api_key: "config-key".to_string(),
            },
        );
        let resolver = ProviderConfigResolver::from_config(&config);

        assert_eq!(
            resolver.resolve_api_key("testprovider"),
            Some("config-key".to_string())
        );
    }

    #[test]
    fn resolve_base_url_from_env() {
        let config = Config::default();
        let mut resolver = ProviderConfigResolver::from_config(&config);
        resolver.env.insert(
            "MYPROVIDER_BASE_URL".to_string(),
            "http://env.local".to_string(),
        );

        assert_eq!(
            resolver.resolve_base_url("myprovider"),
            Some("http://env.local".to_string())
        );
    }

    #[test]
    fn resolve_returns_none_when_missing() {
        let config = Config::default();
        let resolver = ProviderConfigResolver::from_config(&config);

        assert_eq!(resolver.resolve_api_key("nonexistent"), None);
        assert_eq!(resolver.resolve_base_url("nonexistent"), None);
    }
}
