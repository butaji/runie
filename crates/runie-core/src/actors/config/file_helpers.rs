//! File I/O helpers for `RactorConfigActor`.
//!
//! These are blocking operations that run on a separate thread via `spawn_blocking`.
//! Each function wraps load-modify-write under a single exclusive lock to prevent
//! concurrent saves from overwriting each other.
//!
//! Uses `toml_edit` for comment-preserving serialization and `fs2` for cross-process
//! file locking.

use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use anyhow::Context;
use fs2::FileExt;

use crate::config::{McpServer, ModelProvider};
use crate::model::ThinkingLevel;

// ── Lock helpers ───────────────────────────────────────────────────────────────

/// Execute a read-modify-write on a config file under an exclusive lock.
/// The lock is acquired BEFORE reading to prevent truncation races.
/// Uses `toml_edit` for comment-preserving serialization.
fn with_exclusive_lock<F>(path: &Path, mut f: F) -> anyhow::Result<()>
where
    F: FnMut(&mut crate::config::Config),
{
    // Open file before acquiring lock
    let mut file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .with_context(|| format!("failed to open config: {}", path.display()))?;

    // Acquire exclusive lock BEFORE reading (prevents truncation races)
    let _lock = file.lock_exclusive();

    // Read content (file is open, read from start)
    let mut content = String::new();
    file.read_to_string(&mut content).ok();

    // Parse config (handles migrations)
    let mut config = if content.is_empty() {
        crate::config::Config::default()
    } else {
        toml::from_str(&content).unwrap_or_else(|_| crate::config::Config::default())
    };

    // Apply modification
    f(&mut config);

    // Serialize using toml_edit to preserve comments
    let mut doc: toml_edit::DocumentMut = toml_edit::DocumentMut::from(&config);
    let toml_string = doc.to_string();

    // Truncate and write (lock still held)
    file.seek(SeekFrom::Start(0))?;
    file.set_len(toml_string.len() as u64)?;
    file.write_all(toml_string.as_bytes())
        .with_context(|| format!("failed to write config: {}", path.display()))?;
    file.sync_all()
        .with_context(|| format!("failed to sync config: {}", path.display()))?;
    Ok(())
}

// ── File helpers (sync, for use in spawn_blocking) ─────────────────────────────

/// Save a provider entry to the config file.
///
/// API key is stored in keyring first; if keyring is unavailable or retrieval
/// fails, falls back to storing in the config file (legacy mode). The resolution
/// order at runtime is: keyring → env var → config file.
///
/// Uses an exclusive lock for the entire load-modify-write to prevent races.
pub fn save_provider_to_path(
    path: &Path,
    name: &str,
    base_url: &str,
    api_key: &str,
    models: &[String],
) -> anyhow::Result<()> {
    // Try to store api_key in keyring first; if retrieval fails, fall back to config
    let keyring_available = if !api_key.is_empty() {
        crate::auth::set_and_verify_keyring(name, api_key).is_ok()
    } else {
        true
    };

    // Store api_key in config only if keyring is unavailable/unverified
    let stored_api_key = if keyring_available {
        String::new()
    } else {
        api_key.to_owned()
    };

    let n = name.to_owned();
    let b = base_url.to_owned();
    let m = models.to_vec();
    let stored = stored_api_key;
    with_exclusive_lock(path, move |config| {
        let provider_type = config
            .model_providers
            .get(&n)
            .and_then(|p| p.provider_type.clone());
        config.model_providers.insert(
            n.clone(),
            ModelProvider {
                provider_type,
                base_url: b.clone(),
                api_key: stored.clone(),
                models: m.clone(),
            },
        );
    })
}

/// Remove a provider entry from the config file and keyring.
pub fn remove_provider_from_path(path: &Path, name: &str) -> anyhow::Result<()> {
    // Also remove from keyring
    let _ = crate::auth::delete_keyring_entry(name);

    let n = name.to_owned();
    with_exclusive_lock(path, move |config| {
        config.model_providers.remove(&n);
    })
}

/// Set the default provider/model in the config file.
pub fn set_default_model_at_path(path: &Path, provider: &str, model: &str) -> anyhow::Result<()> {
    let p = provider.to_owned();
    let m = model.to_owned();
    with_exclusive_lock(path, move |config| {
        config.provider = Some(p.clone());
        config.model = None;
        config.models.default = Some(m.clone());
        let mp = config
            .model_providers
            .entry(p.clone())
            .or_insert_with(default_empty_provider);
        if !mp.models.contains(&m) && !m.is_empty() {
            mp.models.push(m.clone());
            mp.models.sort();
        }
    })
}

/// Update the model list for a provider.
pub fn set_provider_models_at_path(
    path: &Path,
    name: &str,
    models: &[String],
) -> anyhow::Result<()> {
    let n = name.to_owned();
    let m = models.to_vec();
    with_exclusive_lock(path, move |config| {
        if let Some(mp) = config.model_providers.get_mut(&n) {
            mp.models = m.clone();
        }
    })
}

/// Set the theme name.
pub fn set_theme_at_path(path: &Path, name: &str) -> anyhow::Result<()> {
    let n = name.to_owned();
    with_exclusive_lock(path, move |config| {
        config.theme = Some(n.clone());
    })
}

/// Set vim mode.
pub fn set_vim_mode_at_path(path: &Path, enabled: bool) -> anyhow::Result<()> {
    with_exclusive_lock(path, move |config| {
        config.ui.vim_mode = enabled;
    })
}

/// Set telemetry enabled.
pub fn set_telemetry_at_path(path: &Path, enabled: bool) -> anyhow::Result<()> {
    with_exclusive_lock(path, move |config| {
        config.telemetry.enabled = enabled;
    })
}

/// Set truncation limits.
pub fn set_truncation_at_path(
    path: &Path,
    limits: &crate::config::TruncationSection,
) -> anyhow::Result<()> {
    let l = limits.clone();
    with_exclusive_lock(path, move |config| {
        config.truncation = l.clone();
    })
}

/// Set thinking level.
pub fn set_thinking_level_at_path(path: &Path, level: ThinkingLevel) -> anyhow::Result<()> {
    with_exclusive_lock(path, move |config| {
        config.thinking_level = level;
    })
}

/// Add or update an MCP server in the config file.
pub fn add_mcp_server_to_path(path: &Path, name: &str, server: &McpServer) -> anyhow::Result<()> {
    let n = name.to_owned();
    let s = server.clone();
    with_exclusive_lock(path, move |config| {
        config.mcp.servers.insert(n.clone(), s.clone());
    })
}

/// Remove an MCP server from the config file.
pub fn remove_mcp_server_from_path(path: &Path, name: &str) -> anyhow::Result<()> {
    let n = name.to_owned();
    with_exclusive_lock(path, move |config| {
        config.mcp.servers.remove(&n);
    })
}

fn default_empty_provider() -> ModelProvider {
    ModelProvider {
        provider_type: None,
        base_url: String::new(),
        api_key: String::new(),
        models: Vec::new(),
    }
}
