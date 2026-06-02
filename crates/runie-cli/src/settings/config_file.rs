//! Config file operations - create, read, update skip_onboarding

use std::path::PathBuf;

const DEFAULT_CONFIG: &str = r#"# Runie configuration
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

/// Create default config file at path if it doesn't exist
pub fn create_default_config(path: &std::path::Path) {
    if path.exists() {
        return;
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let default = build_default_config_content();
    std::fs::write(path, default).ok();
}

fn build_default_config_content() -> String {
    DEFAULT_CONFIG.to_string()
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
        Err(_) => create_config_with_skip(skip),
    };

    update_skip_in_content(&config, content, skip)
}

pub fn config_path() -> Option<PathBuf> {
    if let Ok(home) = std::env::var("RUNIE_HOME") {
        return Some(PathBuf::from(home).join("config.toml"));
    }
    dirs::home_dir().map(|h| h.join(".runie/config.toml"))
}

fn create_config_with_skip(skip: bool) -> String {
    let new_content = format!(r#"# Runie configuration

skip_onboarding = {}
"#, skip);
    if let Some(path) = config_path() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
    }
    let _ = std::fs::write(config_path().unwrap(), &new_content);
    new_content
}

fn update_skip_in_content(config: &std::path::Path, content: String, skip: bool) -> bool {
    let new_content = if content.contains("skip_onboarding") {
        replace_existing_skip(&content, skip)
    } else {
        append_skip(&content, skip)
    };
    std::fs::write(config, new_content).is_ok()
}

fn replace_existing_skip(content: &str, skip: bool) -> String {
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
}

fn append_skip(content: &str, skip: bool) -> String {
    format!("{}\nskip_onboarding = {}\n", content.trim(), skip)
}
