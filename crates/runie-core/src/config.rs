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
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
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
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct ModelProvider {
    #[serde(rename = "type")]
    pub provider_type: Option<String>,
    pub base_url: String,
    pub api_key: String,
}

// ============================================================================
// UI Section
// ============================================================================

/// UI configuration section.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
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
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct PromptsSection {
    pub default: Option<String>,
    pub custom: Option<String>,
}

// ============================================================================
// Truncation Section
// ============================================================================

/// Truncation limits for tool output.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
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
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct HooksConfig {
    /// Map of hook event name to list of shell commands to run.
    pub commands: HashMap<String, Vec<String>>,
}

// ============================================================================
// Permissions Section
// ============================================================================

/// Permission policy configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct PermissionsConfig {
    /// Global permission mode: yolo, manual, or auto.
    pub mode: crate::permissions::PermissionMode,
}

impl Default for PermissionsConfig {
    fn default() -> Self {
        Self {
            mode: crate::permissions::PermissionMode::Auto,
        }
    }
}

// ============================================================================
// Main Config
// ============================================================================

/// Canonical config type for `~/.runie/config.toml`.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
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
    /// Fallback provider chain used when the primary provider is unavailable.
    #[serde(default)]
    pub fallback_providers: Vec<String>,
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
    /// Permission policy configuration.
    #[serde(default)]
    pub permissions: PermissionsConfig,
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

    /// Return the provider chain: primary provider followed by fallbacks.
    pub fn provider_chain(&self) -> Vec<&str> {
        let mut chain = Vec::new();
        if let Some(p) = self.provider.as_deref() {
            chain.push(p);
        }
        chain.extend(self.fallback_providers.iter().map(String::as_str));
        chain
    }

    /// Save config to the default path.
    pub fn save(&self) -> anyhow::Result<()> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, toml::to_string_pretty(self)?)?;
        Ok(())
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
        changes
    }
}

/// What changed in the config that the watcher needs to act on.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigChange {
    Model { provider: String, model: String },
    Theme { name: String },
    Keybindings,
}

fn current_config_values(config: &Config) -> (String, String, String) {
    let (default_provider, default_model) = default_provider_model();
    let provider = config
        .provider
        .clone()
        .unwrap_or_else(|| default_provider.to_string());
    let model = config.default_model().unwrap_or(default_model).to_string();
    let theme = config.theme.clone().unwrap_or_else(|| "runie".to_string());
    (provider, model, theme)
}

fn default_provider_model() -> (&'static str, &'static str) {
    if crate::provider_registry::is_mock_enabled() {
        ("mock", "echo")
    } else {
        ("", "")
    }
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
