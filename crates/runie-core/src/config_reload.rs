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

            // Extract provider, model, and theme from config. In production
            // (no RUNIE_MOCK), an absent provider stays empty so the app
            // auto-opens the login dialog instead of silently using mock.
            let default_provider = if crate::provider_registry::is_mock_enabled() {
                "mock"
            } else {
                ""
            };
            let default_model = if crate::provider_registry::is_mock_enabled() {
                "echo"
            } else {
                ""
            };
            let current_provider = config
                .provider
                .clone()
                .unwrap_or_else(|| default_provider.to_string());
            let current_model = config.default_model().unwrap_or(default_model).to_string();
            let current_theme = config.theme.clone().unwrap_or_else(|| "runie".to_string());

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
mod tests;
