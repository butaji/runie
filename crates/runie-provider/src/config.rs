//! Global TOML config (~/.runie/config.toml)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelsSection {
    pub default: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProvider {
    #[serde(rename = "type")]
    pub provider_type: Option<String>,
    pub base_url: String,
    pub api_key: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    pub provider: Option<String>,
    pub model: Option<String>,
    #[serde(default)]
    pub models: ModelsSection,
    #[serde(default)]
    pub model_providers: HashMap<String, ModelProvider>,
}

impl Config {
    pub fn load() -> Self {
        let path = Self::path();
        if !path.exists() {
            return Self::default();
        }
        match std::fs::read_to_string(&path) {
            Ok(text) => toml::from_str(&text).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, toml::to_string_pretty(self)?)?;
        Ok(())
    }

    pub fn path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".runie")
            .join("config.toml")
    }

    pub fn default_model(&self) -> Option<&str> {
        self.models.default.as_deref().or(self.model.as_deref())
    }

    pub fn provider_for_model(&self, full_model: &str) -> Option<&ModelProvider> {
        let prefix = full_model.split('/').next().unwrap_or(full_model);
        self.model_providers.get(prefix)
    }
}

/// Resolves provider configuration from multiple sources with priority:
/// 1. Environment variables
/// 2. .env file in current working directory
/// 3. config.toml model_providers section
#[derive(Debug, Clone, Default)]
pub struct ProviderConfigResolver {
    env: HashMap<String, String>,
    dotenv: HashMap<String, String>,
    config_file: HashMap<String, ModelProvider>,
}

impl ProviderConfigResolver {
    pub fn from_config(config: &Config) -> Self {
        let mut env = HashMap::new();
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

    fn load_dotenv() -> HashMap<String, String> {
        let path = std::env::current_dir().unwrap_or_default().join(".env");
        if !path.exists() {
            return HashMap::new();
        }
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return HashMap::new(),
        };
        let mut map = HashMap::new();
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
