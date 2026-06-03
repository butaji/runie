//! Settings types and configuration structs.

use runie_ai::get_provider_models;
use serde::{Deserialize, Serialize};

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
            permission_mode: PermissionModeConfig::AlwaysApprove,
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
    pub fn load_from_file(path: &std::path::Path) -> Self {
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
