//! Canonical config types for `~/.runie/config.toml`.
//!
//! This module defines the shared TOML schema that both `runie-core`
//! and `runie-provider` consume. It is the single source of truth
//! for the config file format.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

mod layers;
pub mod schema;

// ============================================================================
// Models Section
// ============================================================================

/// Models configuration section.
#[derive(
    Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct ModelsSection {
    /// The default model to use when no model is specified.
    pub default: Option<String>,
    /// Scoped models list (for model selector UI).
    #[serde(default)]
    pub scoped: Option<Vec<String>>,
}

// ============================================================================
// Model Provider
// ============================================================================

/// A provider's configuration entry.
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct ModelProvider {
    #[serde(rename = "type")]
    pub provider_type: Option<String>,
    pub base_url: String,
    pub api_key: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub models: Vec<String>,
}

// ============================================================================
// UI Section
// ============================================================================

/// UI configuration section.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct UiSection {
    pub vim_mode: bool,
}

impl Default for UiSection {
    fn default() -> Self {
        Self { vim_mode: true }
    }
}

// ============================================================================
// Telemetry Section
// ============================================================================

/// Telemetry configuration section.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct TelemetrySection {
    pub enabled: bool,
}

impl Default for TelemetrySection {
    fn default() -> Self {
        Self { enabled: true }
    }
}

// ============================================================================
// Prompts Section
// ============================================================================

/// Prompts configuration section.
#[derive(
    Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(default)]
pub struct PromptsSection {
    pub default: Option<String>,
    pub custom: Option<String>,
}

// ============================================================================
// Truncation Section
// ============================================================================

/// Truncation limits for tool output.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct TruncationSection {
    pub max_lines: usize,
    pub max_bytes: usize,
}

impl Default for TruncationSection {
    fn default() -> Self {
        Self {
            max_lines: 2000,
            max_bytes: 50 * 1024,
        }
    }
}

// ============================================================================
// Hooks Section
// ============================================================================

/// Hook configuration.
#[derive(
    Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(default)]
pub struct HooksConfig {
    /// Map of hook event name to list of shell commands to run.
    pub commands: HashMap<String, Vec<String>>,
}

// ============================================================================
// Permissions Section
// ============================================================================

// ============================================================================
// Main Config
// ============================================================================

/// Canonical config type for `~/.runie/config.toml`.
#[derive(
    Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct Config {
    /// Default provider name.
    pub provider: Option<String>,
    /// Legacy model field (use `[models].default` instead).
    pub model: Option<String>,
    /// Theme name.
    pub theme: Option<String>,
    /// UI settings.
    #[serde(default)]
    pub ui: UiSection,
    /// Model configurations.
    #[serde(default)]
    pub models: ModelsSection,
    /// Provider configurations.
    #[serde(default)]
    pub model_providers: HashMap<String, ModelProvider>,
    /// Telemetry settings.
    #[serde(default)]
    pub telemetry: TelemetrySection,
    /// Prompt templates.
    #[serde(default)]
    pub prompts: PromptsSection,
    /// Truncation settings.
    #[serde(default)]
    pub truncation: TruncationSection,
    /// User-defined keybindings that override defaults.
    /// Parsed from `[keybindings]` table in `config.toml`.
    #[serde(default)]
    pub keybindings: HashMap<String, String>,
    /// Hook commands registered by event name.
    #[serde(default)]
    pub hooks: HooksConfig,
}

impl Config {
    /// Load config from an optional path, falling back to the default path.
    ///
    /// Automatically migrates outdated configs and writes them back.
    pub fn load(path: Option<&std::path::Path>) -> Self {
        let default_path = config_path();
        let path = path.unwrap_or(&default_path);
        if !path.exists() {
            return Self::default();
        }
        match std::fs::read_to_string(path) {
            Ok(text) => {
                let mut value: toml::Value = match toml::from_str(&text) {
                    Ok(v) => v,
                    Err(_) => return Self::default(),
                };
                match crate::config_migrate::migrate_with_path(&mut value, Some(path.to_path_buf()))
                {
                    Ok(true) => {
                        let _ = crate::config_migrate::backup_config(path);
                        if let Ok(migrated) = toml::to_string(&value) {
                            let _ = std::fs::write(path, migrated);
                        }
                    }
                    Ok(false) => {}
                    Err(_) => {}
                }
                let s = toml::to_string(&value).unwrap_or_default();
                toml::from_str(&s).unwrap_or_default()
            }
            Err(_) => Self::default(),
        }
    }

    /// Load config asynchronously, moving blocking file IO off the runtime.
    pub async fn load_async(path: Option<PathBuf>) -> Self {
        tokio::task::spawn_blocking(move || Self::load(path.as_deref()))
            .await
            .unwrap_or_default()
    }

    /// Load configuration from layered sources: defaults → global config →
    /// local project config → environment variables.
    pub fn load_layers() -> Self {
        layers::load_layers()
    }

    /// Layered config loader with explicit paths (useful for tests).
    pub fn load_layers_from_paths(global: PathBuf, local: PathBuf) -> Self {
        layers::load_layers_from_paths(global, local)
    }

    /// Load config and validate it against the JSON schema.
    ///
    /// Returns an error describing all validation failures.
    pub fn load_strict(path: Option<&Path>) -> Result<Self, Vec<String>> {
        let config = Self::load(path);
        config.validate().map(|_| config)
    }

    /// Validate this config against the canonical JSON schema.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let value = serde_json::to_value(self)
            .map_err(|e| vec![format!("config serialization failed: {e}")])?;
        Self::validate_value(&value)
    }

    /// Validate a raw TOML value against the canonical JSON schema.
    pub fn validate_toml(value: &toml::Value) -> Result<(), Vec<String>> {
        let json = serde_json::to_value(value)
            .map_err(|e| vec![format!("config serialization failed: {e}")])?;
        Self::validate_value(&json)
    }

    fn validate_value(value: &serde_json::Value) -> Result<(), Vec<String>> {
        let schema = schema::schema_value();
        let validator =
            jsonschema::validator_for(&schema).map_err(|e| vec![format!("invalid schema: {e}")])?;
        let errors: Vec<String> = validator
            .iter_errors(value)
            .map(|e| e.to_string())
            .collect();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Save config to the default path.
    pub fn save(&self) -> anyhow::Result<()> {
        self.save_to(&config_path())
    }

    /// Save config to an explicit path.
    pub fn save_to(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, toml::to_string_pretty(self)?)?;
        Ok(())
    }

    /// Save config without blocking the async runtime.
    /// Outside a runtime this behaves like [`save`].
    pub fn save_nonblocking(&self) {
        self.save_nonblocking_to(&config_path());
    }

    /// Save config to the given path without blocking the async runtime.
    pub fn save_nonblocking_to(&self, path: &Path) {
        let path = path.to_path_buf();
        let text = match toml::to_string_pretty(self) {
            Ok(t) => t,
            Err(e) => {
                tracing::error!("failed to serialize config: {e}");
                return;
            }
        };
        crate::async_io::run_blocking_if_runtime(move || {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Err(e) = std::fs::write(&path, text) {
                tracing::error!("failed to write config: {e}");
            }
        });
    }

    /// Get the default model (from `[models].default` or legacy `model` field).
    pub fn default_model(&self) -> Option<&str> {
        self.models.default.as_deref().or(self.model.as_deref())
    }

    /// Get the list of scoped models.
    pub fn scoped_models(&self) -> Option<&Vec<String>> {
        self.models.scoped.as_ref()
    }

    /// Check if telemetry is enabled.
    pub fn telemetry_enabled(&self) -> bool {
        self.telemetry.enabled
    }

    /// Get the prompts section.
    pub fn prompts(&self) -> &PromptsSection {
        &self.prompts
    }

    /// Check if vim mode is enabled.
    pub fn vim_mode(&self) -> bool {
        self.ui.vim_mode
    }

    /// Get the provider for a specific model.
    pub fn provider_for_model(&self, full_model: &str) -> Option<&ModelProvider> {
        let prefix = full_model.split('/').next().unwrap_or(full_model);
        self.model_providers.get(prefix)
    }

    /// List configured providers sorted by name.
    pub fn configured_providers(&self) -> Vec<(String, String, Vec<String>)> {
        let mut result: Vec<_> = self
            .model_providers
            .iter()
            .map(|(name, p)| (name.clone(), p.base_url.clone(), p.models.clone()))
            .collect();
        result.sort_by(|a, b| a.0.cmp(&b.0));
        result
    }

    /// Return the configured models for a provider.
    pub fn models_for_provider(&self, provider: &str) -> Vec<String> {
        self.model_providers
            .get(provider)
            .map(|p| p.models.clone())
            .unwrap_or_default()
    }

    /// Return the first configured model for a provider, if any.
    pub fn first_model_for_provider(&self, provider: &str) -> Option<String> {
        self.models_for_provider(provider).into_iter().next()
    }

    /// Resolve the default provider/model pair from this config.
    ///
    /// Falls back through: explicit `provider` + saved models, first configured
    /// provider's first model, and finally empty strings when nothing is set.
    pub fn resolve_default_model(&self) -> (String, String) {
        if crate::provider::is_mock_enabled() {
            return ("mock".into(), "echo".into());
        }
        if let Some(provider) = self.provider.as_deref().filter(|p| !p.is_empty()) {
            let model = self
                .first_model_for_provider(provider)
                .or_else(|| self.default_model().map(String::from))
                .unwrap_or_default();
            return (provider.to_string(), model);
        }
        let mut providers: Vec<_> = self.model_providers.iter().collect();
        providers.sort_by_key(|(k, _)| *k);
        if let Some((provider, mp)) = providers.into_iter().next() {
            if let Some(model) = mp.models.first() {
                return (provider.clone(), model.clone());
            }
        }
        (String::new(), String::new())
    }

    /// Classify what changed between two configs.
    pub fn classify_change(&self, prev: &Config) -> Vec<ConfigChange> {
        let mut changes = Vec::new();
        let new_vals = current_config_values(self);
        let old_vals = current_config_values(prev);

        if new_vals.0 != old_vals.0 || new_vals.1 != old_vals.1 {
            changes.push(ConfigChange::Model {
                provider: new_vals.0,
                model: new_vals.1,
            });
        }
        if new_vals.2 != old_vals.2 {
            changes.push(ConfigChange::Theme { name: new_vals.2 });
        }
        if self.keybindings != prev.keybindings {
            changes.push(ConfigChange::Keybindings);
        }
        if self.model_providers != prev.model_providers {
            changes.push(ConfigChange::Credentials);
        }
        changes
    }
}

/// What changed in the config that the watcher needs to act on.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigChange {
    Model { provider: String, model: String },
    Theme { name: String },
    Keybindings,
    Credentials,
}

fn current_config_values(config: &Config) -> (String, String, String) {
    let (provider, model) = config.resolve_default_model();
    let theme = config.theme.clone().unwrap_or_else(|| "runie".to_string());
    (provider, model, theme)
}

/// Get the default config file path.
pub fn config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".runie")
        .join("config.toml")
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests;
