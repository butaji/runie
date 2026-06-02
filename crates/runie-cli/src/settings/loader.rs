//! Settings loading and resolution logic.

use runie_ai::get_provider_models;
use std::path::{Path, PathBuf};

pub use super::config::{
    BulletStyle, CliConfig, ExecuteBlockConfig, HeaderStyle, PermissionModeConfig,
    RunieConfig, ScrollbarConfig, ScrollbackBlocks, ScrollbackConfig, ScrollbackLayout,
    ThinkingBlockConfig, ToolBlockConfig, UiConfig, AnimationConfig,
};

/// Resolved settings from all sources
#[derive(Debug, Clone, serde::Serialize)]
pub struct Settings {
    pub model: String,
    pub provider: String,
    pub api_key: Option<String>,
    pub max_turns: usize,
    pub enable_thinking: bool,
    pub shell: String,
    /// Whether a config file was loaded (vs using defaults/no config)
    pub config_loaded: bool,
    /// Skip onboarding flow permanently (persisted to config)
    pub skip_onboarding: bool,
    /// UI/Animation/Scrollback configuration
    pub runie_config: RunieConfig,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            model: "gpt-4o".to_string(),
            provider: "openai".to_string(),
            api_key: None,
            max_turns: 10,
            enable_thinking: true,
            shell: std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string()),
            config_loaded: false,
            skip_onboarding: false,
            runie_config: RunieConfig::default(),
        }
    }
}

fn merge_api_key_fallback(settings: &mut Settings) {
    // Try OPENAI_API_KEY if no RUNIE_API_KEY was set
    if settings.api_key.is_none() {
        if let Ok(val) = std::env::var("OPENAI_API_KEY") {
            settings.api_key = Some(val);
            return;
        }
    }
    // Try MINIMAX_API_KEY as another fallback
    if settings.api_key.is_none() {
        if let Ok(val) = std::env::var("MINIMAX_API_KEY") {
            settings.api_key = Some(val);
        }
    }
}

impl Settings {
    /// Load settings with layered resolution
    pub fn load() -> Self {
        let mut settings = Self::default();

        // Layer 2: Global config (RUNIE_HOME/config.toml or ~/.runie/config.toml)
        if let Some(global) = runie_dir().map(|p| p.join("config.toml")) {
            if global.exists() {
                settings.merge_file(&global);
                settings.runie_config = RunieConfig::load_from_file(&global);
                settings.config_loaded = true;
            }
        }

        // Layer 3: Project config .runie/config.toml
        if let Ok(cwd) = std::env::current_dir() {
            let project = cwd.join(".runie/config.toml");
            if project.exists() {
                settings.merge_file(&project);
                settings.runie_config = RunieConfig::load_from_file(&project);
                settings.config_loaded = true;
            }
        }

        // Layer 4: Environment variables
        settings.merge_env();

        settings
    }

    /// Merge settings from a TOML file
    fn merge_file(&mut self, path: &Path) {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(file_settings) = toml::from_str::<FileSettings>(&content) {
                file_settings.apply_to(self);
            }
        }
    }

    /// Merge settings from environment variables
    fn merge_env(&mut self) {
        // Standard RUNIE_* env vars
        if let Ok(val) = std::env::var("RUNIE_MODEL") {
            self.model = val;
        }
        if let Ok(val) = std::env::var("RUNIE_PROVIDER") {
            self.provider = val;
        }
        if let Ok(val) = std::env::var("RUNIE_API_KEY") {
            self.api_key = Some(val);
        }
        if let Ok(val) = std::env::var("RUNIE_MAX_TURNS") {
            if let Ok(v) = val.parse() {
                self.max_turns = v;
            }
        }
        if let Ok(val) = std::env::var("RUNIE_ENABLE_THINKING") {
            self.enable_thinking = val.to_lowercase() != "false";
        }
        if let Ok(val) = std::env::var("RUNIE_SHELL") {
            self.shell = val;
        }
        if let Ok(val) = std::env::var("RUNIE_SKIP_ONBOARDING") {
            self.skip_onboarding = val.to_lowercase() == "true";
        }
        // Legacy/provider-specific API key fallback
        merge_api_key_fallback(self);
    }

    /// Merge settings from CLI arguments
    #[allow(dead_code)]
    pub fn merge_cli(&mut self, cli: &CliSettings) {
        if let Some(ref m) = cli.model {
            self.model = m.clone();
        }
        if let Some(ref p) = cli.provider {
            self.provider = p.clone();
        }
        if let Some(ref k) = cli.api_key {
            self.api_key = Some(k.clone());
        }
        if let Some(v) = cli.max_turns {
            self.max_turns = v;
        }
        if let Some(v) = cli.enable_thinking {
            self.enable_thinking = v;
        }
        if let Some(ref s) = cli.shell {
            self.shell = s.clone();
        }
    }

    /// Validate model against static registry
    #[allow(dead_code)]
    pub fn validate_model(&self) -> bool {
        get_provider_models(&self.provider)
            .map(|models| models.iter().any(|m| m.id == self.model))
            .unwrap_or(false)
    }
}

/// CLI-level settings (only fields that can be set via CLI)
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct CliSettings {
    pub model: Option<String>,
    pub provider: Option<String>,
    pub api_key: Option<String>,
    pub max_turns: Option<usize>,
    pub enable_thinking: Option<bool>,
    pub shell: Option<String>,
}

/// Internal struct for parsing TOML config files
#[derive(Debug, serde::Deserialize)]
struct FileSettings {
    model: Option<String>,
    provider: Option<String>,
    api_key: Option<String>,
    max_turns: Option<usize>,
    enable_thinking: Option<bool>,
    shell: Option<String>,
    skip_onboarding: Option<bool>,
}

impl FileSettings {
    fn apply_to(&self, settings: &mut Settings) {
        if let Some(ref v) = self.model {
            settings.model = v.clone();
        }
        if let Some(ref v) = self.provider {
            settings.provider = v.clone();
        }
        if let Some(ref v) = self.api_key {
            settings.api_key = Some(v.clone());
        }
        if let Some(v) = self.max_turns {
            settings.max_turns = v;
        }
        if let Some(v) = self.enable_thinking {
            settings.enable_thinking = v;
        }
        if let Some(ref v) = self.shell {
            settings.shell = v.clone();
        }
        if let Some(v) = self.skip_onboarding {
            settings.skip_onboarding = v;
        }
    }
}

/// Runie config directory paths
/// Checks RUNIE_HOME env var first, then falls back to ~/.runie
pub fn runie_dir() -> Option<PathBuf> {
    if let Ok(home) = std::env::var("RUNIE_HOME") {
        return Some(PathBuf::from(home));
    }
    dirs::home_dir().map(|h| h.join(".runie"))
}

pub fn sessions_dir() -> Option<PathBuf> {
    runie_dir().map(|p| p.join("sessions"))
}

pub fn themes_dir() -> Option<PathBuf> {
    runie_dir().map(|p| p.join("themes"))
}

pub fn skills_dir() -> Option<PathBuf> {
    runie_dir().map(|p| p.join("skills"))
}

pub fn agent_dir() -> Option<PathBuf> {
    runie_dir().map(|p| p.join("agent"))
}

/// Ensure all runie directories exist
pub fn ensure_dirs() {
    if let Some(dir) = runie_dir() {
        std::fs::create_dir_all(&dir).ok();
    }
    if let Some(dir) = agent_dir() {
        std::fs::create_dir_all(&dir).ok();
    }
    if let Some(dir) = sessions_dir() {
        std::fs::create_dir_all(&dir).ok();
    }
    if let Some(dir) = themes_dir() {
        std::fs::create_dir_all(&dir).ok();
    }
    if let Some(dir) = skills_dir() {
        std::fs::create_dir_all(&dir).ok();
    }
}
