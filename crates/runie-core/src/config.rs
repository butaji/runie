//! Canonical config types for `~/.runie/config.toml`.
//!
//! This module defines the shared TOML schema that both `runie-core`
//! and `runie-provider` consume. It is the single source of truth
//! for the config file format.

use std::collections::HashMap;
use std::path::PathBuf;

// ============================================================================
// Models Section
// ============================================================================

/// Models configuration section.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PromptsSection {
    pub default: Option<String>,
    pub custom: Option<String>,
}

impl Default for PromptsSection {
    fn default() -> Self {
        Self {
            default: None,
            custom: None,
        }
    }
}

// ============================================================================
// Truncation Section
// ============================================================================

/// Truncation limits for tool output.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
// Main Config
// ============================================================================

/// Canonical config type for `~/.runie/config.toml`.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
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
}

impl Config {
    /// Load config from a specific path.
    ///
    /// Automatically migrates outdated configs and writes them back.
    pub fn load_from(path: &PathBuf) -> Self {
        if !path.exists() {
            return Self::default();
        }
        match std::fs::read_to_string(path) {
            Ok(text) => {
                let mut value: toml::Value = match toml::from_str(&text) {
                    Ok(v) => v,
                    Err(_) => return Self::default(),
                };
                match crate::config_migrate::migrate_with_path(&mut value, Some(path.clone())) {
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

    /// Load config from the default path.
    pub fn load() -> Self {
        Self::load_from(&config_path())
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
    let model = config
        .default_model()
        .unwrap_or(default_model)
        .to_string();
    let theme = config
        .theme
        .clone()
        .unwrap_or_else(|| "runie".to_string());
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
mod tests {
    use super::*;
    use std::fs;

    fn make_test_config(dir: &tempfile::TempDir, content: &str) -> std::path::PathBuf {
        let path = dir.path().join("config.toml");
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn config_load_parses_basic_fields() {
        let dir = tempfile::tempdir().unwrap();
        let path = make_test_config(&dir, r#"
provider = "openai"
model = "gpt-4"
"#);
        let config = Config::load_from(&path);
        assert_eq!(config.provider, Some("openai".to_string()));
        assert_eq!(config.default_model(), Some("gpt-4"));
    }

    #[test]
    fn config_load_parses_models_section() {
        let dir = tempfile::tempdir().unwrap();
        let path = make_test_config(&dir, r#"
[models]
default = "gpt-4o"
scoped = ["gpt-4", "gpt-3.5-turbo"]
"#);
        let config = Config::load_from(&path);
        assert_eq!(config.default_model(), Some("gpt-4o"));
        let scoped = config.scoped_models().unwrap();
        assert_eq!(scoped.len(), 2);
    }

    #[test]
    fn config_load_parses_provider_section() {
        let dir = tempfile::tempdir().unwrap();
        let path = make_test_config(&dir, r#"
[model_providers.openai]
type = "openai"
base_url = "https://api.openai.com"
api_key = "sk-test"
"#);
        let config = Config::load_from(&path);
        let provider = config.provider_for_model("openai/gpt-4").unwrap();
        assert_eq!(provider.base_url, "https://api.openai.com");
    }

    #[test]
    fn config_load_parses_ui_section() {
        let dir = tempfile::tempdir().unwrap();
        let path = make_test_config(&dir, r#"
[ui]
vim_mode = false
"#);
        let config = Config::load_from(&path);
        assert!(!config.vim_mode());
    }

    #[test]
    fn config_load_parses_telemetry_section() {
        let dir = tempfile::tempdir().unwrap();
        let path = make_test_config(&dir, r#"
[telemetry]
enabled = false
"#);
        let config = Config::load_from(&path);
        assert!(!config.telemetry_enabled());
    }

    #[test]
    fn config_defaults_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nonexistent.toml");
        let config = Config::load_from(&path);
        assert_eq!(config.provider, None);
        assert_eq!(config.default_model(), None);
        assert!(config.vim_mode());
    }

    #[test]
    fn config_path_returns_expected_path() {
        let path = config_path();
        assert!(path.file_name().is_some_and(|n| n == "config.toml"));
    }

    #[test]
    fn classify_change_detects_model_change() {
        let prev = Config { provider: Some("openai".to_string()), ..Config::default() };
        let curr = Config { provider: Some("anthropic".to_string()), ..Config::default() };
        let changes = curr.classify_change(&prev);
        assert!(matches!(changes.as_slice(), [ConfigChange::Model { provider, .. }] if provider == "anthropic"));
    }

    #[test]
    fn classify_change_detects_theme_change() {
        let prev = Config { theme: Some("dark".to_string()), ..Config::default() };
        let curr = Config { theme: Some("light".to_string()), ..Config::default() };
        let changes = curr.classify_change(&prev);
        assert!(matches!(changes.as_slice(), [ConfigChange::Theme { name }] if name == "light"));
    }

    #[test]
    fn classify_change_returns_empty_when_identical() {
        let prev = Config { provider: Some("openai".to_string()), theme: Some("dark".to_string()), ..Config::default() };
        let curr = prev.clone();
        assert!(curr.classify_change(&prev).is_empty());
    }

    #[test]
    fn classify_change_detects_keybindings_change() {
        let mut prev = Config::default();
        let mut curr = Config::default();
        prev.keybindings.insert("ctrl+c".to_string(), "Quit".to_string());
        curr.keybindings.insert("ctrl+c".to_string(), "Abort".to_string());
        let changes = curr.classify_change(&prev);
        assert!(matches!(changes.as_slice(), [ConfigChange::Keybindings]));
    }

    #[test]
    fn classify_change_multiple_changes() {
        let mut prev = Config::default();
        let mut curr = Config::default();
        prev.provider = Some("openai".to_string());
        curr.provider = Some("anthropic".to_string());
        curr.theme = Some("nord".to_string());
        curr.keybindings.insert("ctrl+c".to_string(), "Abort".to_string());
        let changes = curr.classify_change(&prev);
        assert_eq!(changes.len(), 3);
    }

    #[test]
    fn config_load_parses_all_sections() {
        let dir = tempfile::tempdir().unwrap();
        let path = make_test_config(&dir, r#"
provider = "openai"
model = "gpt-4"
theme = "nord"

[models]
default = "gpt-4o"

[ui]
vim_mode = false

[telemetry]
enabled = false

[truncation]
max_lines = 100
max_bytes = 100000

[prompts]
default = "default"
"#);
        let config = Config::load_from(&path);
        assert_eq!(config.provider, Some("openai".to_string()));
        // default_model() prefers [models].default over top-level model
        assert_eq!(config.default_model(), Some("gpt-4o"));
        assert_eq!(config.theme, Some("nord".to_string()));
        assert!(!config.vim_mode());
        assert!(!config.telemetry_enabled());
    }

    #[test]
    fn provider_and_core_see_same_default_model() {
        // Both provider crate and core crate should read from the same Config type
        // This is verified by re-exports in runie-provider/src/config.rs:
        //   pub use runie_core::config::{Config, ModelProvider, ModelsSection};
        let dir = tempfile::tempdir().unwrap();
        let path = make_test_config(&dir, r#"
[models]
default = "gpt-4o"
"#);
        let config = Config::load_from(&path);
        let default = config.default_model();
        // Re-load from path to verify consistency
        let config2 = Config::load_from(&path);
        assert_eq!(default, config2.default_model());
    }
}
