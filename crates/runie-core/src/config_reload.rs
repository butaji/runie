//! Config file watcher for hot reload
//!
//! Watches config.toml for changes and emits SwitchModel events
//! when the provider or model configuration changes.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::interval;

use crate::event::Event;

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

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct Config {
    pub provider: Option<String>,
    model: Option<String>,
    pub theme: Option<String>,
    #[serde(default)]
    models: ModelsSection,
    #[serde(default)]
    #[allow(dead_code)]
    model_providers: HashMap<String, serde_json::Value>,
    #[serde(default)]
    telemetry: TelemetrySection,
    #[serde(default)]
    prompts: PromptsSection,
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
}

/// Start a config file watcher that monitors for changes and emits SwitchModel events.
///
/// Returns a tokio::JoinHandle that can be used to stop the watcher.
///
/// The watcher:
/// - Polls the config file every 2 seconds
/// - Compares the provider/model with the last known values
/// - Emits SwitchModel events when they change
pub fn spawn_config_watcher(
    event_tx: mpsc::Sender<Event>,
    config_path: PathBuf,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        // Track last known provider/model/theme to detect changes
        let mut last_provider: Option<String> = None;
        let mut last_model: Option<String> = None;
        let mut last_theme: Option<String> = None;

        let mut poll_interval = interval(Duration::from_secs(2));

        loop {
            poll_interval.tick().await;

            // Load current config from the specified path
            let config = Config::load_from(&config_path);

            // Extract provider, model, and theme from config
            let current_provider = config.provider.clone().unwrap_or_else(|| "mock".to_string());
            let current_model = config.default_model().unwrap_or("echo").to_string();
            let current_theme = config.theme.clone().unwrap_or_else(|| "silkcircuit-neon".to_string());

            // Check if provider changed
            let provider_changed = match &last_provider {
                Some(prev) => prev != &current_provider,
                None => true, // First run, always report change
            };

            // Check if model changed
            let model_changed = match &last_model {
                Some(prev) => prev != &current_model,
                None => true, // First run, always report change
            };

            // Check if theme changed
            let theme_changed = match &last_theme {
                Some(prev) => prev != &current_theme,
                None => true, // First run, always report change
            };

            // Emit when provider or model changes
            if provider_changed || model_changed {
                let evt = Event::SwitchModel {
                    provider: current_provider.clone(),
                    model: current_model.clone(),
                };
                if event_tx.send(evt).await.is_err() {
                    // Channel closed, exit watcher
                    break;
                }
            }

            // Emit when theme changes
            if theme_changed {
                let evt = Event::SwitchTheme {
                    name: current_theme.clone(),
                };
                if event_tx.send(evt).await.is_err() {
                    // Channel closed, exit watcher
                    break;
                }
            }

            // Update tracking
            last_provider = Some(current_provider);
            last_model = Some(current_model);
            last_theme = Some(current_theme);
        }
    })
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
    use super::*;
    use crate::model::AppState;
    use crate::Event;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn config_changed_applies_provider() {
        // Layer 2: Verify SwitchModel event updates AppState
        let mut state = AppState::default();
        
        // Initial defaults
        assert_eq!(state.current_provider, "mock");
        assert_eq!(state.current_model, "echo");
        
        // Send SwitchModel event
        state.update(Event::SwitchModel {
            provider: "anthropic".to_string(),
            model: "claude-3-sonnet".to_string(),
        });
        
        // Verify provider and model are updated
        assert_eq!(state.current_provider, "anthropic");
        assert_eq!(state.current_model, "claude-3-sonnet");
        
        // Verify a system message was added to indicate the change
        let has_switch_msg = state.messages.iter().any(|m| {
            m.role == crate::model::Role::System &&
            m.content.contains("Switched to anthropic/claude-3-sonnet")
        });
        assert!(has_switch_msg, "Should add system message on model switch");
    }

    #[test]
    fn config_theme_change_applies_theme() {
        let mut state = AppState::default();
        assert_eq!(state.theme_name, "silkcircuit-neon");

        state.update(Event::SwitchTheme {
            name: "dracula".to_string(),
        });

        assert_eq!(state.theme_name, "dracula");
        let has_theme_msg = state.messages.iter().any(|m| {
            m.role == crate::model::Role::System &&
            m.content.contains("Theme switched to 'dracula'")
        });
        assert!(has_theme_msg, "Should add system message on theme switch");
    }

    #[tokio::test]
    async fn config_watcher_detects_initial_change() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");

        // Create initial config with explicit provider/model
        fs::write(&config_path, r#"
provider = "openai"
model = "gpt-4"

[model_providers.openai]
type = "openai"
base_url = "https://api.openai.com"
api_key = "test"
"#).unwrap();

        let (tx, mut rx) = mpsc::channel::<Event>(10);

        // Spawn watcher
        let handle = spawn_config_watcher(tx, config_path.clone());

        // Wait for the watcher to pick up the initial config
        // Give it time to load and compare (2 poll intervals = 4 seconds)
        tokio::time::sleep(Duration::from_secs(4)).await;

        // Check that a SwitchModel event was emitted for initial load
        let evt = tokio::time::timeout(Duration::from_secs(1), rx.recv()).await;
        assert!(evt.is_ok(), "Should receive SwitchModel event");
        assert!(matches!(evt.unwrap(), Some(Event::SwitchModel { .. })));

        // Clean up
        handle.abort();
    }

    #[tokio::test]
    async fn config_watcher_parses_toml_changes() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");

        // Create initial config
        fs::write(&config_path, r#"
provider = "openai"
model = "gpt-4"

[model_providers.openai]
type = "openai"
base_url = "https://api.openai.com"
api_key = "test"
"#).unwrap();

        let (tx, mut rx) = mpsc::channel::<Event>(10);
        let handle = spawn_config_watcher(tx, config_path.clone());

        // Wait for initial load
        tokio::time::sleep(Duration::from_secs(4)).await;

        // Drain any initial events
        while rx.try_recv().is_ok() {}

        // Now change the config
        fs::write(&config_path, r#"
provider = "anthropic"
model = "claude-3"

[model_providers.anthropic]
type = "anthropic"
base_url = "https://api.anthropic.com"
api_key = "test"
"#).unwrap();

        // Wait for the watcher to detect the change
        tokio::time::sleep(Duration::from_secs(3)).await;

        let evt = tokio::time::timeout(Duration::from_secs(1), rx.recv()).await;
        assert!(evt.is_ok(), "Should receive SwitchModel event");

        if let Ok(Some(Event::SwitchModel { provider, model })) = evt {
            assert_eq!(provider, "anthropic");
            assert_eq!(model, "claude-3");
        } else {
            panic!("Expected SwitchModel event");
        }

        handle.abort();
    }

    #[test]
    fn config_path_returns_expected_path() {
        let path = config_path();
        assert!(path.components().next().is_some(), "Path should not be empty");
        assert!(path.file_name().is_some_and(|n| n == "config.toml"), 
                "Path should end with config.toml");
    }

    #[test]
    fn config_load_parses_toml() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");

        // Write a config file
        fs::write(&config_path, r#"
provider = "test-provider"
model = "test-model"

[model_providers.test-provider]
type = "test"
base_url = "http://localhost"
api_key = "secret"
"#).unwrap();

        // Load the config (migration moves top-level model → models.default)
        let config = Config::load_from(&config_path);

        assert_eq!(config.provider, Some("test-provider".to_string()));
        assert_eq!(config.default_model(), Some("test-model"));
    }

    #[test]
    fn config_load_defaults_when_missing() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("nonexistent.toml");

        let config = Config::load_from(&config_path);

        assert_eq!(config.provider, None);
        assert_eq!(config.model, None);
        assert_eq!(config.default_model(), None);
    }

    #[test]
    fn config_theme_field_emits_switch_theme() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");

        fs::write(&config_path, r#"
theme = "dracula"
"#).unwrap();

        let config = Config::load_from(&config_path);
        assert_eq!(config.theme, Some("dracula".to_string()));
    }

    #[test]
    fn config_load_parses_scoped_models() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");

        fs::write(&config_path, r#"
provider = "openai"

[models]
scoped = ["gpt-4o", "claude-3-sonnet", "gemini-1.5-pro"]
"#).unwrap();

        let config = Config::load_from(&config_path);
        let scoped = config.scoped_models().expect("should have scoped models");
        assert_eq!(scoped.len(), 3);
        assert_eq!(scoped[0], "gpt-4o");
        assert_eq!(scoped[1], "claude-3-sonnet");
        assert_eq!(scoped[2], "gemini-1.5-pro");
    }

    #[test]
    fn config_load_scoped_models_missing_is_none() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");

        fs::write(&config_path, r#"
provider = "openai"
model = "gpt-4"
"#).unwrap();

        let config = Config::load_from(&config_path);
        assert!(config.scoped_models().is_none());
    }

    #[test]
    fn config_load_uses_default_model_from_models_section() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");

        fs::write(&config_path, r#"
provider = "openai"
model = "gpt-3.5"

[models]
default = "gpt-4"

[model_providers.openai]
type = "openai"
base_url = "https://api.openai.com"
api_key = "test"
"#).unwrap();

        let config = Config::load_from(&config_path);

        // models.default already existed, so migration should NOT overwrite it
        assert_eq!(config.default_model(), Some("gpt-4"));
    }
}
