//! Settings manager with layered resolution (CLI args > env > project config > global config > defaults)

use runie_ai::ModelRegistry;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Resolved settings from all sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub model: String,
    pub provider: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub max_turns: usize,
    pub temperature: f32,
    pub theme: String,
    pub keybindings: Option<Keybindings>,
    pub auto_save: bool,
    pub compact_threshold: usize,
    pub tool_mode: String,
    pub enable_thinking: bool,
    pub shell: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keybindings {
    pub submit: Option<String>,
    pub new_line: Option<String>,
    pub exit: Option<String>,
    pub sidebar: Option<String>,
    pub command_palette: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            model: "gpt-4o".to_string(),
            provider: "openai".to_string(),
            api_key: None,
            base_url: None,
            max_turns: 10,
            temperature: 0.7,
            theme: "crush_grok".to_string(),
            keybindings: None,
            auto_save: true,
            compact_threshold: 8000,
            tool_mode: "parallel".to_string(),
            enable_thinking: true,
            shell: std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string()),
        }
    }
}

impl Settings {
    /// Load settings with layered resolution
    pub fn load() -> Self {
        let mut settings = Self::default();

        // Layer 2: Global config ~/.runie/config.toml
        if let Some(home) = dirs::home_dir() {
            let global = home.join(".runie/config.toml");
            if global.exists() {
                settings.merge_file(&global);
            }
        }

        // Layer 3: Project config .runie/config.toml
        if let Ok(cwd) = std::env::current_dir() {
            let project = cwd.join(".runie/config.toml");
            if project.exists() {
                settings.merge_file(&project);
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
        if let Ok(val) = std::env::var("RUNIE_MODEL") {
            self.model = val;
        }
        if let Ok(val) = std::env::var("RUNIE_PROVIDER") {
            self.provider = val;
        }
        if let Ok(val) = std::env::var("RUNIE_API_KEY") {
            self.api_key = Some(val);
        }
        if let Ok(val) = std::env::var("RUNIE_BASE_URL") {
            self.base_url = Some(val);
        }
        if let Ok(val) = std::env::var("RUNIE_MAX_TURNS") {
            if let Ok(v) = val.parse() {
                self.max_turns = v;
            }
        }
        if let Ok(val) = std::env::var("RUNIE_TEMPERATURE") {
            if let Ok(v) = val.parse() {
                self.temperature = v;
            }
        }
        if let Ok(val) = std::env::var("RUNIE_THEME") {
            self.theme = val;
        }
        if let Ok(val) = std::env::var("RUNIE_AUTO_SAVE") {
            self.auto_save = val.to_lowercase() != "false";
        }
        if let Ok(val) = std::env::var("RUNIE_COMPACT_THRESHOLD") {
            if let Ok(v) = val.parse() {
                self.compact_threshold = v;
            }
        }
        if let Ok(val) = std::env::var("RUNIE_TOOL_MODE") {
            self.tool_mode = val;
        }
        if let Ok(val) = std::env::var("RUNIE_ENABLE_THINKING") {
            self.enable_thinking = val.to_lowercase() != "false";
        }
        if let Ok(val) = std::env::var("RUNIE_SHELL") {
            self.shell = val;
        }
        // Legacy env vars
        if let Ok(val) = std::env::var("OPENAI_API_KEY") {
            if self.api_key.is_none() {
                self.api_key = Some(val);
            }
        }
    }

    /// Merge settings from CLI arguments
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
        if let Some(ref u) = cli.base_url {
            self.base_url = Some(u.clone());
        }
        if let Some(v) = cli.max_turns {
            self.max_turns = v;
        }
        if let Some(v) = cli.temperature {
            self.temperature = v;
        }
        if let Some(ref t) = cli.theme {
            self.theme = t.clone();
        }
        if let Some(ref kb) = cli.keybindings {
            self.keybindings = Some(kb.clone());
        }
        if let Some(v) = cli.auto_save {
            self.auto_save = v;
        }
        if let Some(v) = cli.compact_threshold {
            self.compact_threshold = v;
        }
        if let Some(ref m) = cli.tool_mode {
            self.tool_mode = m.clone();
        }
        if let Some(v) = cli.enable_thinking {
            self.enable_thinking = v;
        }
        if let Some(ref s) = cli.shell {
            self.shell = s.clone();
        }
    }

    /// Validate model against registry
    pub fn validate_model(&self) -> bool {
        let registry = ModelRegistry::new();
        registry.get(&self.model).is_some()
    }
}

/// CLI-level settings (only fields that can be set via CLI)
#[derive(Debug, Clone, Default)]
pub struct CliSettings {
    pub model: Option<String>,
    pub provider: Option<String>,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub max_turns: Option<usize>,
    pub temperature: Option<f32>,
    pub theme: Option<String>,
    pub keybindings: Option<Keybindings>,
    pub auto_save: Option<bool>,
    pub compact_threshold: Option<usize>,
    pub tool_mode: Option<String>,
    pub enable_thinking: Option<bool>,
    pub shell: Option<String>,
}

/// Internal struct for parsing TOML config files
#[derive(Debug, Deserialize)]
struct FileSettings {
    model: Option<String>,
    provider: Option<String>,
    api_key: Option<String>,
    base_url: Option<String>,
    max_turns: Option<usize>,
    temperature: Option<f32>,
    theme: Option<String>,
    keybindings: Option<Keybindings>,
    auto_save: Option<bool>,
    compact_threshold: Option<usize>,
    tool_mode: Option<String>,
    enable_thinking: Option<bool>,
    shell: Option<String>,
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
        if let Some(ref v) = self.base_url {
            settings.base_url = Some(v.clone());
        }
        if let Some(v) = self.max_turns {
            settings.max_turns = v;
        }
        if let Some(v) = self.temperature {
            settings.temperature = v;
        }
        if let Some(ref v) = self.theme {
            settings.theme = v.clone();
        }
        if let Some(ref kb) = self.keybindings {
            settings.keybindings = Some(kb.clone());
        }
        if let Some(v) = self.auto_save {
            settings.auto_save = v;
        }
        if let Some(v) = self.compact_threshold {
            settings.compact_threshold = v;
        }
        if let Some(ref v) = self.tool_mode {
            settings.tool_mode = v.clone();
        }
        if let Some(v) = self.enable_thinking {
            settings.enable_thinking = v;
        }
        if let Some(ref v) = self.shell {
            settings.shell = v.clone();
        }
    }
}

/// Runie config directory paths
pub fn runie_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".runie"))
}

pub fn config_path() -> Option<PathBuf> {
    runie_dir().map(|p| p.join("config.toml"))
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

/// Create default config file at path if it doesn't exist
pub fn create_default_config(path: &Path) {
    if path.exists() {
        return;
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let default = r#"# Runie configuration
# See https://github.com/... for documentation

model = "gpt-4o"
provider = "openai"
max_turns = 10
temperature = 0.7
theme = "crush_grok"
auto_save = true
compact_threshold = 8000
tool_mode = "parallel"
enable_thinking = true
"#;
    std::fs::write(path, default).ok();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = Settings::default();
        assert_eq!(settings.model, "gpt-4o");
        assert_eq!(settings.provider, "openai");
        assert_eq!(settings.max_turns, 10);
        assert_eq!(settings.temperature, 0.7);
    }

    #[test]
    fn test_merge_env() {
        std::env::set_var("RUNIE_MODEL", "claude-3-opus");
        std::env::set_var("RUNIE_MAX_TURNS", "20");
        let settings = Settings::load();
        assert_eq!(settings.model, "claude-3-opus");
        assert_eq!(settings.max_turns, 20);
        std::env::remove_var("RUNIE_MODEL");
        std::env::remove_var("RUNIE_MAX_TURNS");
    }

    #[test]
    fn test_merge_cli() {
        let mut settings = Settings::default();
        let cli = CliSettings {
            model: Some("anthropic".to_string()),
            max_turns: Some(5),
            ..Default::default()
        };
        settings.merge_cli(&cli);
        assert_eq!(settings.model, "anthropic");
        assert_eq!(settings.max_turns, 5);
    }
}
