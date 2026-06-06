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
