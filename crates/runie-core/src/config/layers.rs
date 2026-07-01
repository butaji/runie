//! Layered configuration loading using Figment.
//!
//! Configuration precedence (lowest to highest):
//! 1. Defaults (Config::default())
//! 2. Global config (~/.runie/config.toml)
//! 3. Project config (.runie/config.toml)
//! 4. Environment variables (RUNIE_PROVIDER, RUNIE_MODEL, RUNIE_THEME)

use std::path::PathBuf;

use figment::providers::{Env, Format, Serialized, Toml};
use figment::Figment;

use crate::config::{config_path, Config};

/// Load configuration from layered sources: defaults → global config →
/// local project config → environment variables.
pub fn load_layers() -> Config {
    load_layers_from_paths(config_path(), PathBuf::from(".runie").join("config.toml"))
}

/// Layered config loader with explicit paths (useful for tests).
pub fn load_layers_from_paths(global: PathBuf, local: PathBuf) -> Config {
    // Build Figment with layered sources
    let mut figment = Figment::new();

    // 1. Defaults
    figment = figment.merge(Serialized::defaults(Config::default()));

    // 2. Global config (only if file exists)
    if global.exists() {
        figment = figment.merge(Toml::file(&global));
    }

    // 3. Project config (only if file exists)
    if local.exists() {
        figment = figment.merge(Toml::file(&local));
    }

    // 4. Environment variables with RUNIE_ prefix
    figment = figment.merge(Env::prefixed("RUNIE_"));

    // Extract config from Figment
    let mut config: Config = figment
        .extract()
        .unwrap_or_else(|e| {
            tracing::warn!("failed to extract config from figment: {}", e);
            Config::default()
        });

    // Apply manual env overrides for known keys (RUNIE_PROVIDER, RUNIE_MODEL, RUNIE_THEME)
    // These map to Config fields that may have different names in the TOML
    if let Ok(provider) = std::env::var("RUNIE_PROVIDER") {
        config.provider = Some(provider);
    }
    if let Ok(model) = std::env::var("RUNIE_MODEL") {
        config.model = Some(model);
    }
    if let Ok(theme) = std::env::var("RUNIE_THEME") {
        config.theme = Some(theme);
    }

    config
}
