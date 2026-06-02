//! Settings module - layered configuration resolution.
//!
//! Layers (highest wins):
//! 1. CLI arguments
//! 2. Environment variables
//! 3. Project config (.runie/config.toml)
//! 4. Global config (RUNIE_HOME/config.toml or ~/.runie/config.toml)
//! 5. Defaults

pub mod config;
pub mod config_file;
pub mod loader;
pub mod tests;

pub use config::{
    AnimationConfig, BulletStyle, CliConfig, ExecuteBlockConfig, HeaderStyle,
    PermissionModeConfig, RunieConfig, ScrollbarConfig, ScrollbackBlocks,
    ScrollbackConfig, ScrollbackLayout, ThinkingBlockConfig, ToolBlockConfig, UiConfig,
};
pub use config_file::{create_default_config, set_skip_onboarding, config_path};
pub use loader::{
    agent_dir, ensure_dirs, sessions_dir, skills_dir, themes_dir,
    runie_dir, CliSettings, Settings,
};
