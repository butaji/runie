//! Settings manager with layered resolution (CLI args > env > project config > global config > defaults)

use runie_ai::get_provider_models;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ─── UI Config ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UiConfig {
    pub permission_mode: PermissionModeConfig,
    pub vim_mode: bool,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PermissionModeConfig {
    Ask,
    AlwaysApprove,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            permission_mode: PermissionModeConfig::Ask,
            vim_mode: false,
        }
    }
}

// ─── Animation Config ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AnimationConfig {
    pub fps: u32,
    pub wave_rows: u32,
}

impl Default for AnimationConfig {
    fn default() -> Self {
        Self {
            fps: 30,
            wave_rows: 32,
        }
    }
}

// ─── Scrollback Config ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScrollbackConfig {
    pub layout: ScrollbackLayout,
    pub scrollbar: ScrollbarConfig,
    pub blocks: ScrollbackBlocks,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScrollbackLayout {
    pub outer_vpad: u16,
    pub outer_hpad_left: u16,
    pub outer_hpad_right: u16,
    pub block_pad_left: u16,
    pub block_pad_right: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScrollbarConfig {
    pub enabled: bool,
    pub gap_left: u16,
    pub gap_right: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScrollbackBlocks {
    pub thinking: ThinkingBlockConfig,
    pub tool: ToolBlockConfig,
    pub execute: ExecuteBlockConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ThinkingBlockConfig {
    pub accent_enabled: bool,
    pub animate: bool,
    pub truncated_lines: u32,
    pub header: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolBlockConfig {
    pub muted_collapsed: bool,
    pub dim_details: bool,
    pub bullet: BulletStyle,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BulletStyle {
    Dot,
    SmallCircle,
    Circle,
    SmallTriangle,
    Triangle,
    Diamond,
    None,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExecuteBlockConfig {
    pub first_lines: u32,
    pub last_lines: u32,
    pub accent_enabled: bool,
    pub header_style: HeaderStyle,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum HeaderStyle {
    Label,
    Shell,
}

impl Default for ScrollbackConfig {
    fn default() -> Self {
        Self {
            layout: ScrollbackLayout::default(),
            scrollbar: ScrollbarConfig::default(),
            blocks: ScrollbackBlocks::default(),
        }
    }
}

impl Default for ScrollbackLayout {
    fn default() -> Self {
        Self {
            outer_vpad: 1,
            outer_hpad_left: 2,
            outer_hpad_right: 2,
            block_pad_left: 2,
            block_pad_right: 2,
        }
    }
}

impl Default for ScrollbarConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            gap_left: 0,
            gap_right: 0,
        }
    }
}

impl Default for ScrollbackBlocks {
    fn default() -> Self {
        Self {
            thinking: ThinkingBlockConfig::default(),
            tool: ToolBlockConfig::default(),
            execute: ExecuteBlockConfig::default(),
        }
    }
}

impl Default for ThinkingBlockConfig {
    fn default() -> Self {
        Self {
            accent_enabled: true,
            animate: true,
            truncated_lines: 3,
            header: true,
        }
    }
}

impl Default for ToolBlockConfig {
    fn default() -> Self {
        Self {
            muted_collapsed: true,
            dim_details: true,
            bullet: BulletStyle::Diamond,
        }
    }
}

impl Default for ExecuteBlockConfig {
    fn default() -> Self {
        Self {
            first_lines: 2,
            last_lines: 3,
            accent_enabled: true,
            header_style: HeaderStyle::Label,
        }
    }
}

// ─── CLI Config ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CliConfig {
    pub auto_update: bool,
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            auto_update: true,
        }
    }
}

// ─── Full RunieConfig ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RunieConfig {
    pub ui: UiConfig,
    pub animation: AnimationConfig,
    pub scrollback: ScrollbackConfig,
    pub cli: CliConfig,
}

impl Default for RunieConfig {
    fn default() -> Self {
        Self {
            ui: UiConfig::default(),
            animation: AnimationConfig::default(),
            scrollback: ScrollbackConfig::default(),
            cli: CliConfig::default(),
        }
    }
}

impl RunieConfig {
    /// Load config from file, merging with defaults
    pub fn load_from_file(path: &Path) -> Self {
        let mut config = Self::default();
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(file_config) = toml::from_str::<RunieConfig>(&content) {
                // Merge with defaults - file values override
                config.ui.permission_mode = file_config.ui.permission_mode;
                config.ui.vim_mode = file_config.ui.vim_mode;
                config.animation.fps = file_config.animation.fps;
                config.animation.wave_rows = file_config.animation.wave_rows;
                // Layout
                config.scrollback.layout = file_config.scrollback.layout;
                config.scrollback.scrollbar = file_config.scrollback.scrollbar;
                config.scrollback.blocks = file_config.scrollback.blocks;
                config.cli.auto_update = file_config.cli.auto_update;
            }
        }
        config
    }
}

/// Parse permission_mode string to enum
impl PermissionModeConfig {
    pub fn from_str(s: &str) -> Self {
        match s {
            "always-approve" => PermissionModeConfig::AlwaysApprove,
            _ => PermissionModeConfig::Ask,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            PermissionModeConfig::Ask => "ask",
            PermissionModeConfig::AlwaysApprove => "always-approve",
        }
    }
}

/// Resolved settings from all sources
#[derive(Debug, Clone, Serialize)]
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
#[derive(Debug, Deserialize)]
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
# See https://github.com/your-repo/runie for documentation

# Model configuration
model = "gpt-4o"
provider = "openai"
max_turns = 10
enable_thinking = true
skip_onboarding = false

[ui]
# Permission mode: "ask" or "always-approve"
permission_mode = "ask"
# Enable vim keybindings for navigation
vim_mode = false

[animation]
# Animation frame rate (1-60)
fps = 30
# Rows per wave cycle for thinking animation
wave_rows = 32

[scrollback.layout]
outer_vpad = 1
outer_hpad_left = 2
outer_hpad_right = 2
block_pad_left = 2
block_pad_right = 2

[scrollback.scrollbar]
enabled = true
gap_left = 0
gap_right = 0

[scrollback.blocks.thinking]
accent_enabled = true
animate = true
truncated_lines = 3
header = true

[scrollback.blocks.tool]
muted_collapsed = true
dim_details = true
bullet = "diamond"

[scrollback.blocks.execute]
first_lines = 2
last_lines = 3
accent_enabled = true
header_style = "label"

[cli]
auto_update = true
"#;
    std::fs::write(path, default).ok();
}

/// Set skip_onboarding flag in config file
/// Returns true if successful
pub fn set_skip_onboarding(skip: bool) -> bool {
    let config = match config_path() {
        Some(p) => p,
        None => return false,
    };

    let content = match std::fs::read_to_string(&config) {
        Ok(c) => c,
        Err(_) => {
            // Config doesn't exist, create with skip_onboarding
            let new_content = format!(r#"# Runie configuration

skip_onboarding = {}
"#, skip);
            if let Some(parent) = config.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            return std::fs::write(&config, new_content).is_ok();
        }
    };

    // Check if skip_onboarding already exists
    let new_content = if content.contains("skip_onboarding") {
        content.lines()
            .map(|line| {
                if line.trim().starts_with("skip_onboarding") {
                    format!("skip_onboarding = {}", skip)
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        // Append skip_onboarding to existing config
        format!("{}\nskip_onboarding = {}\n", content.trim(), skip)
    };

    std::fs::write(&config, new_content).is_ok()
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

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = Settings::default();
        assert_eq!(settings.model, "gpt-4o");
        assert_eq!(settings.provider, "openai");
        assert_eq!(settings.max_turns, 10);
        assert_eq!(settings.enable_thinking, true);
    }

    /// Guard that saves env vars, clears them, sets test values, and restores on drop
    struct EnvGuard {
        originals: Vec<(String, Option<String>)>,
    }

    impl EnvGuard {
        fn new() -> Self {
            Self { originals: Vec::new() }
        }

        fn save_and_clear(&mut self, var: &str) {
            let original = std::env::var(var).ok();
            std::env::remove_var(var);
            self.originals.push((var.to_string(), original));
        }

        fn set(&self, var: &str, val: &str) {
            std::env::set_var(var, val);
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (var, original) in self.originals.drain(..) {
                std::env::remove_var(&var);
                if let Some(val) = original {
                    std::env::set_var(&var, val);
                }
            }
        }
    }

    #[test]
    fn test_merge_env() {
        let mut guard = EnvGuard::new();
        guard.save_and_clear("RUNIE_MODEL");
        guard.save_and_clear("RUNIE_MAX_TURNS");
        guard.save_and_clear("RUNIE_HOME"); // Prevent reading external config
        guard.set("RUNIE_MODEL", "claude-3-opus");
        guard.set("RUNIE_MAX_TURNS", "20");

        let settings = Settings::load();
        assert_eq!(settings.model, "claude-3-opus");
        assert_eq!(settings.max_turns, 20);
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

    #[test]
    fn test_settings_default_values() {
        let settings = Settings::default();
        assert_eq!(settings.model, "gpt-4o");
        assert_eq!(settings.provider, "openai");
        assert_eq!(settings.max_turns, 10);
        assert_eq!(settings.enable_thinking, true);
        assert_eq!(settings.shell, std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string()));
    }

    #[test]
    fn test_settings_layered_resolution() {
        let mut guard = EnvGuard::new();
        guard.save_and_clear("RUNIE_MODEL");
        guard.save_and_clear("RUNIE_PROVIDER");
        guard.save_and_clear("RUNIE_MAX_TURNS");

        // Start with defaults
        let mut settings = Settings::default();
        assert_eq!(settings.model, "gpt-4o");
        assert_eq!(settings.provider, "openai");
        assert_eq!(settings.max_turns, 10);

        // Layer: env vars override defaults
        std::env::set_var("RUNIE_MODEL", "claude-3-opus");
        std::env::set_var("RUNIE_PROVIDER", "anthropic");
        std::env::set_var("RUNIE_MAX_TURNS", "25");
        settings.merge_env();
        assert_eq!(settings.model, "claude-3-opus");
        assert_eq!(settings.provider, "anthropic");
        assert_eq!(settings.max_turns, 25);

        // Layer: CLI overrides env
        let cli = CliSettings {
            model: Some("gpt-5".to_string()),
            provider: Some("openai".to_string()),
            max_turns: Some(50),
            ..Default::default()
        };
        settings.merge_cli(&cli);
        assert_eq!(settings.model, "gpt-5");
        assert_eq!(settings.provider, "openai");
        assert_eq!(settings.max_turns, 50);
    }

    #[test]
    fn test_dev_folder_sets_runie_home() {
        // RUNIE_HOME env var should be respected
        std::env::set_var("RUNIE_HOME", "./tmp");
        let dir = runie_dir();
        assert!(dir.is_some());
        assert_eq!(dir.unwrap(), PathBuf::from("./tmp"));
        std::env::remove_var("RUNIE_HOME");

        // Without RUNIE_HOME, falls back to ~/.runie
        std::env::remove_var("RUNIE_HOME");
        let dir = runie_dir();
        assert!(dir.is_some());
        // Should contain .runie
        assert!(dir.unwrap().to_string_lossy().contains(".runie"));
    }

    #[test]
    fn test_validate_model_true() {
        let settings = Settings {
            model: "gpt-4o".to_string(),
            provider: "openai".to_string(),
            ..Default::default()
        };
        assert!(settings.validate_model());
    }

    #[test]
    fn test_validate_model_false() {
        let settings = Settings {
            model: "invalid-model-xyz".to_string(),
            provider: "openai".to_string(),
            ..Default::default()
        };
        assert!(!settings.validate_model());
    }

    #[test]
    fn test_settings_merge_cli_overrides() {
        let mut settings = Settings::default();
        // Default values
        assert_eq!(settings.model, "gpt-4o");
        assert_eq!(settings.provider, "openai");
        assert_eq!(settings.max_turns, 10);
        assert_eq!(settings.enable_thinking, true);

        // CLI overrides everything
        let cli = CliSettings {
            model: Some("gpt-5".to_string()),
            provider: Some("anthropic".to_string()),
            api_key: Some("sk-test123".to_string()),
            max_turns: Some(100),
            enable_thinking: Some(false),
            shell: Some("/bin/zsh".to_string()),
        };
        settings.merge_cli(&cli);

        assert_eq!(settings.model, "gpt-5");
        assert_eq!(settings.provider, "anthropic");
        assert_eq!(settings.api_key, Some("sk-test123".to_string()));
        assert_eq!(settings.max_turns, 100);
        assert_eq!(settings.enable_thinking, false);
        assert_eq!(settings.shell, "/bin/zsh");
    }
}
