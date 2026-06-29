//! File I/O helpers for `RactorConfigActor`.
//!
//! These are blocking operations that run on a separate thread via `spawn_blocking`.

use std::path::Path;

use crate::config::{McpServer, ModelProvider};
use crate::model::ThinkingLevel;

// ── File helpers (sync, for use in spawn_blocking) ─────────────────────────────

/// Save a provider entry to the config file.
pub fn save_provider_to_path(
    path: &Path,
    name: &str,
    base_url: &str,
    api_key: &str,
    models: &[String],
) -> anyhow::Result<()> {
    let mut config = crate::config::Config::load(Some(path));
    let provider_type = config
        .model_providers
        .get(name)
        .and_then(|p| p.provider_type.clone());
    config.model_providers.insert(
        name.into(),
        ModelProvider {
            provider_type,
            base_url: base_url.into(),
            api_key: api_key.into(),
            models: models.into(),
        },
    );
    config.save_to(path)
}

/// Remove a provider entry from the config file.
pub fn remove_provider_from_path(path: &Path, name: &str) -> anyhow::Result<()> {
    let mut config = crate::config::Config::load(Some(path));
    config.model_providers.remove(name);
    config.save_to(path)
}

/// Set the default provider/model in the config file.
pub fn set_default_model_at_path(path: &Path, provider: &str, model: &str) -> anyhow::Result<()> {
    let mut config = crate::config::Config::load(Some(path));
    config.provider = Some(provider.into());
    config.model = None;
    config.models.default = Some(model.into());
    let mp = config
        .model_providers
        .entry(provider.into())
        .or_insert_with(default_empty_provider);
    if !mp.models.contains(&model.into()) && !model.is_empty() {
        mp.models.push(model.into());
        mp.models.sort();
    }
    config.save_to(path)
}

/// Update the model list for a provider.
pub fn set_provider_models_at_path(
    path: &Path,
    name: &str,
    models: &[String],
) -> anyhow::Result<()> {
    let mut config = crate::config::Config::load(Some(path));
    if let Some(mp) = config.model_providers.get_mut(name) {
        mp.models = models.to_vec();
    }
    config.save_to(path)
}

/// Set the theme name.
pub fn set_theme_at_path(path: &Path, name: &str) -> anyhow::Result<()> {
    let mut config = crate::config::Config::load(Some(path));
    config.theme = Some(name.to_owned());
    config.save_to(path)
}

/// Set vim mode.
pub fn set_vim_mode_at_path(path: &Path, enabled: bool) -> anyhow::Result<()> {
    let mut config = crate::config::Config::load(Some(path));
    config.ui.vim_mode = enabled;
    config.save_to(path)
}

/// Set telemetry enabled.
pub fn set_telemetry_at_path(path: &Path, enabled: bool) -> anyhow::Result<()> {
    let mut config = crate::config::Config::load(Some(path));
    config.telemetry.enabled = enabled;
    config.save_to(path)
}

/// Set truncation limits.
pub fn set_truncation_at_path(
    path: &Path,
    limits: &crate::config::TruncationSection,
) -> anyhow::Result<()> {
    let mut config = crate::config::Config::load(Some(path));
    config.truncation = limits.clone();
    config.save_to(path)
}

/// Set thinking level.
pub fn set_thinking_level_at_path(path: &Path, level: ThinkingLevel) -> anyhow::Result<()> {
    let mut config = crate::config::Config::load(Some(path));
    config.thinking_level = level;
    config.save_to(path)
}

/// Add or update an MCP server in the config file.
pub fn add_mcp_server_to_path(path: &Path, name: &str, server: &McpServer) -> anyhow::Result<()> {
    let mut config = crate::config::Config::load(Some(path));
    config.mcp.servers.insert(name.to_owned(), server.clone());
    config.save_to(path)
}

/// Remove an MCP server from the config file.
pub fn remove_mcp_server_from_path(path: &Path, name: &str) -> anyhow::Result<()> {
    let mut config = crate::config::Config::load(Some(path));
    config.mcp.servers.remove(name);
    config.save_to(path)
}

fn default_empty_provider() -> ModelProvider {
    ModelProvider {
        provider_type: None,
        base_url: String::new(),
        api_key: String::new(),
        models: Vec::new(),
    }
}
