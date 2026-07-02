//! Keyring operations for auth storage.
//!
//! Provides low-level keyring CRUD operations using the OS keychain.
//! Backed by `OsKeyringStore` from the `store_trait` module.

use std::collections::HashMap;

use secrecy::ExposeSecret;

use crate::auth::AuthToken;

use super::store_trait::{KeyringStore, OsKeyringStore};

/// Number of characters shown in a token preview when logging mismatches.
const TOKEN_PREVIEW_LENGTH: usize = 8;

/// Set a provider token directly in the keyring (no instance state needed).
/// This is used by the config migration to move plaintext keys to keyring.
pub fn set_keyring_value(provider: &str, token: &str) -> anyhow::Result<()> {
    set_keyring(provider, token)
}

/// Set a provider token in the keyring and verify it can be retrieved.
///
/// Returns `Ok(())` only if both `set_password` and `get_password` succeed
/// and the retrieved value matches the input. This guards against keyring
/// backends that silently fail retrieval (e.g., macOS Keychain access issues).
pub fn set_and_verify_keyring(provider: &str, token: &str) -> anyhow::Result<()> {
    let store = OsKeyringStore::new();
    store.set(provider, token)?;
    match store.get(provider) {
        Ok(Some(stored)) if stored.expose_secret() == token => Ok(()),
        Ok(Some(stored)) => {
            let s = stored.expose_secret();
            Err(anyhow::anyhow!(
                "keyring returned different token (len={}): {:?}",
                s.len(),
                &s[..s.len().min(TOKEN_PREVIEW_LENGTH)]
            ))
        }
        Ok(None) => Err(anyhow::anyhow!("keyring retrieval returned None after set")),
        Err(e) => Err(anyhow::anyhow!("keyring retrieval failed: {e}")),
    }
}

/// Delete a provider token from the keyring.
pub fn delete_keyring_entry(provider: &str) -> anyhow::Result<()> {
    delete_keyring(provider)
}

/// Set a token in the OS keyring.
pub fn set_keyring(provider: &str, token: &str) -> anyhow::Result<()> {
    OsKeyringStore::new().set(provider, token)
}

/// Get a token from the OS keyring.
pub fn get_keyring(provider: &str) -> anyhow::Result<String> {
    OsKeyringStore::new()
        .get(provider)?
        .map(|s| s.expose_secret().clone())
        .ok_or_else(|| anyhow::anyhow!("keyring: no entry for '{provider}'"))
}

/// Delete a token from the OS keyring.
pub fn delete_keyring(provider: &str) -> anyhow::Result<()> {
    OsKeyringStore::new().delete(provider)
}

/// Load all known provider tokens from the keyring.
pub fn load_all_from_keyring() -> anyhow::Result<HashMap<String, AuthToken>> {
    let mut tokens = HashMap::new();
    let common_providers = ["openai", "anthropic", "google", "groq", "mistral", "cohere", "xai"];
    for provider in common_providers {
        if let Some(token) = OsKeyringStore::new().get(provider)? {
            tokens.insert(
                provider.to_owned(),
                AuthToken {
                    provider: provider.to_owned(),
                    token,
                    expires_at: None,
                },
            );
        }
    }
    Ok(tokens)
}

// ---------------------------------------------------------------------------
// Migration from legacy XOR-encoded file
// ---------------------------------------------------------------------------

fn default_auth_path() -> Option<std::path::PathBuf> {
    dirs::data_dir().map(|d| d.join("runie").join("auth.json"))
}

/// Migrate legacy `~/.runie/auth.json` to keyring.
/// This is a no-op if keyring is unavailable.
pub fn migrate_legacy_auth() -> anyhow::Result<()> {
    let Some(path) = default_auth_path() else {
        return Ok(());
    };

    if !path.exists() {
        return Ok(());
    }

    let json = std::fs::read_to_string(&path)?;
    let raw: serde_json::Value = serde_json::from_str(&json).unwrap_or(serde_json::json!({}));

    if let Some(obj) = raw.as_object() {
        for (provider, val) in obj {
            if let Some(token_str) = val.get("token").and_then(|v| v.as_str()) {
                if !token_str.is_empty() {
                    if let Err(e) = set_keyring(provider, token_str) {
                        tracing::warn!("failed to migrate token for {}: {}", provider, e);
                    }
                }
            }
        }
    }

    let backup = path.with_extension("json.bak");
    if let Err(e) = std::fs::rename(&path, &backup) {
        tracing::debug!("could not rename legacy auth file: {}", e);
    }

    Ok(())
}
