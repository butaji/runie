//! Config file types for hot reload.

use std::collections::HashMap;
use std::path::PathBuf;

// Duplicated from runie_provider::Config to avoid circular dependency
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct ModelsSection {
    default: Option<String>,
    scoped: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct TelemetrySection {
    #[serde(default)]
    enabled: bool,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct PromptsSection {
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub custom: Option<String>,
}

/// Truncation limits for tool output. See `[truncation]` in `config.toml`.
/// Defaults match the documented limits in the agent crate; if those change
/// here, also update `runie-agent::truncate::DEFAULT_MAX_LINES/_BYTES`.
#[derive(Debug, Clone, serde::Deserialize)]
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

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default)]
pub struct UiSection {
    pub vim_mode: bool,
}

impl Default for UiSection {
    fn default() -> Self {
        Self { vim_mode: true }
    }
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct Config {
    pub provider: Option<String>,
    model: Option<String>,
    pub theme: Option<String>,
    #[serde(default)]
    ui: UiSection,
    #[serde(default)]
    models: ModelsSection,
    #[serde(default)]
    #[allow(dead_code)]
    model_providers: HashMap<String, serde_json::Value>,
    #[serde(default)]
    telemetry: TelemetrySection,
    #[serde(default)]
    prompts: PromptsSection,
    #[serde(default)]
    pub truncation: TruncationSection,
}

impl Config {
    /// Load config from a specific path.
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
                match crate::config_migrate::migrate(&mut value) {
                    Ok(true) => {
                        // Backup old config before overwriting
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

    pub fn default_model(&self) -> Option<&str> {
        self.models.default.as_deref().or(self.model.as_deref())
    }

    pub fn scoped_models(&self) -> Option<&Vec<String>> {
        self.models.scoped.as_ref()
    }

    pub fn telemetry_enabled(&self) -> bool {
        self.telemetry.enabled
    }

    pub fn prompts(&self) -> &PromptsSection {
        &self.prompts
    }

    pub fn vim_mode(&self) -> bool {
        self.ui.vim_mode
    }
}

/// Get the default config file path
pub fn config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".runie")
        .join("config.toml")
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{config_path, Config};
    use crate::event::Event;
    use crate::model::AppState;

    #[test]
    fn config_changed_applies_provider() {
        let mut state = AppState::default();

        let (def_provider, def_model) = if crate::provider_registry::is_mock_enabled() {
            ("mock", "echo")
        } else {
            ("", "")
        };
        assert_eq!(state.config.current_provider, def_provider);
        assert_eq!(state.config.current_model, def_model);

        state.update(Event::SwitchModel {
            provider: "anthropic".to_string(),
            model: "claude-3-sonnet".to_string(),
        });

        assert_eq!(state.config.current_provider, "anthropic");
        assert_eq!(state.config.current_model, "claude-3-sonnet");
        assert_eq!(
            state.transient_message,
            Some("Switched to anthropic/claude-3-sonnet".into())
        );
        assert_eq!(
            state.transient_level,
            Some(crate::event::TransientLevel::Success)
        );
    }

    #[test]
    fn config_theme_change_applies_theme() {
        let mut state = AppState::default();
        assert_eq!(state.config.theme_name, "runie");

        state.update(Event::SwitchTheme {
            name: "dracula".to_string(),
        });

        assert_eq!(state.config.theme_name, "dracula");
        assert_eq!(
            state.transient_message,
            Some("Theme switched to 'dracula'".into())
        );
        assert_eq!(
            state.transient_level,
            Some(crate::event::TransientLevel::Success)
        );
    }

    #[test]
    fn config_theme_unchanged_does_not_notify() {
        let mut state = AppState::default();
        assert_eq!(state.config.theme_name, "runie");
        state.transient_message = None;
        state.transient_level = None;

        state.update(Event::SwitchTheme {
            name: "runie".to_string(),
        });

        assert_eq!(state.config.theme_name, "runie");
        assert!(state.transient_message.is_none());
    }

    #[test]
    fn config_model_unchanged_does_not_notify() {
        let mut state = AppState::default();
        let (def_provider, def_model) = if crate::provider_registry::is_mock_enabled() {
            ("mock", "echo")
        } else {
            ("", "")
        };
        assert_eq!(state.config.current_provider, def_provider);
        assert_eq!(state.config.current_model, def_model);
        state.transient_message = None;
        state.transient_level = None;

        state.update(Event::SwitchModel {
            provider: def_provider.to_string(),
            model: def_model.to_string(),
        });

        assert_eq!(state.config.current_provider, def_provider);
        assert_eq!(state.config.current_model, def_model);
        assert!(state.transient_message.is_none());
    }

    #[test]
    fn config_path_returns_expected_path() {
        let path = config_path();
        assert!(
            path.components().next().is_some(),
            "Path should not be empty"
        );
        assert!(
            path.file_name().is_some_and(|n| n == "config.toml"),
            "Path should end with config.toml"
        );
    }

    #[test]
    fn config_load_parses_toml() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.toml");

        fs::write(
            &config_path,
            r#"
provider = "test-provider"
model = "test-model"

[model_providers.test-provider]
type = "test"
base_url = "http://localhost"
api_key = "secret"
"#,
        )
        .unwrap();

        let config = Config::load_from(&config_path);

        assert_eq!(config.provider, Some("test-provider".to_string()));
        assert_eq!(config.default_model(), Some("test-model"));
    }

    #[test]
    fn config_load_defaults_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("nonexistent.toml");

        let config = Config::load_from(&config_path);

        assert_eq!(config.provider, None);
        assert_eq!(config.model, None);
        assert_eq!(config.default_model(), None);
    }

    #[test]
    fn config_theme_field_emits_switch_theme() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.toml");

        fs::write(
            &config_path,
            r#"
theme = "dracula"
"#,
        )
        .unwrap();

        let config = Config::load_from(&config_path);
        assert_eq!(config.theme, Some("dracula".to_string()));
    }

    #[test]
    fn config_load_parses_scoped_models() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.toml");

        fs::write(
            &config_path,
            r#"
provider = "openai"

[models]
scoped = ["gpt-4o", "claude-3-sonnet", "gemini-1.5-pro"]
"#,
        )
        .unwrap();

        let config = Config::load_from(&config_path);
        let scoped = config.scoped_models().expect("should have scoped models");
        assert_eq!(scoped.len(), 3);
        assert_eq!(scoped[0], "gpt-4o");
        assert_eq!(scoped[1], "claude-3-sonnet");
        assert_eq!(scoped[2], "gemini-1.5-pro");
    }

    #[test]
    fn config_load_scoped_models_missing_is_none() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.toml");

        fs::write(
            &config_path,
            r#"
provider = "openai"
model = "gpt-4"
"#,
        )
        .unwrap();

        let config = Config::load_from(&config_path);
        assert!(config.scoped_models().is_none());
    }

    #[test]
    fn config_vim_mode_default_true() {
        let config = Config::default();
        assert!(config.vim_mode());
    }

    #[test]
    fn config_vim_mode_true() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.toml");

        fs::write(
            &config_path,
            r#"
[ui]
vim_mode = true
"#,
        )
        .unwrap();

        let config = Config::load_from(&config_path);
        assert!(config.vim_mode());
    }

    #[test]
    fn reload_all_applies_vim_mode() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        fs::write(
            &config_path,
            r#"
[ui]
vim_mode = true
"#,
        )
        .unwrap();

        let mut state = AppState::default();
        state.config.vim_mode = false;
        assert!(!state.config.vim_mode);
        let config = Config::load_from(&config_path);
        if config.vim_mode() {
            state.config.vim_mode = true;
        }
        assert!(state.config.vim_mode);
    }

    #[test]
    fn config_load_uses_default_model_from_models_section() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.toml");

        fs::write(
            &config_path,
            r#"
provider = "openai"
model = "gpt-3.5"

[models]
default = "gpt-4"

[model_providers.openai]
type = "openai"
base_url = "https://api.openai.com"
api_key = "test"
"#,
        )
        .unwrap();

        let config = Config::load_from(&config_path);

        assert_eq!(config.default_model(), Some("gpt-4"));
    }
}
