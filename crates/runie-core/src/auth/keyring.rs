//! Keyring operations for auth storage.
//!
//! Provides low-level keyring CRUD operations using the OS keychain.
//! Backed by `OsKeyringStore` from the `store_trait` module.

use std::collections::HashMap;

use secrecy::ExposeSecret;

use crate::auth::AuthToken;

use super::store_trait::{KeyringStore, OsKeyringStore};

/// Format the keyring mismatch error. Intentionally contains only lengths — the
/// stored token is never sliced or interpolated, so a mismatch cannot leak part
/// of the secret into logs or crash reports.
fn format_keyring_mismatch(stored_len: usize, expected_len: usize) -> String {
    format!("keyring returned a different token (stored len={stored_len}, expected len={expected_len})")
}

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
            let stored_len = stored.expose_secret().len();
            Err(anyhow::anyhow!(
                "{}",
                format_keyring_mismatch(stored_len, token.len())
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
    let common_providers = [
        "openai",
        "anthropic",
        "google",
        "groq",
        "mistral",
        "cohere",
        "xai",
    ];
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
    migrate_legacy_auth_from(&path)
}

/// Migrate a specific legacy `auth.json` file to the keyring, then rename it to
/// `auth.json.bak` so the plaintext secret is no longer at the well-known path.
///
/// Keyring writes are best-effort (a headless/CI host may have no keyring); the
/// rename away from the plaintext path happens regardless, so a partially-
/// migrated host still stops leaving secrets on disk. Exposed for tests.
pub fn migrate_legacy_auth_from(path: &std::path::Path) -> anyhow::Result<()> {
    if !path.exists() {
        return Ok(());
    }

    let json = std::fs::read_to_string(path)?;
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
    if let Err(e) = std::fs::rename(path, &backup) {
        tracing::debug!("could not rename legacy auth file: {}", e);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mismatch_message_contains_only_lengths() {
        let msg = format_keyring_mismatch(40, 38);
        assert!(msg.contains("stored len=40"), "{msg}");
        assert!(msg.contains("expected len=38"), "{msg}");
    }

    /// A mismatch must never echo any token characters (no prefix/slice),
    /// otherwise a keyring backend bug could leak part of a secret into logs.
    #[test]
    fn mismatch_message_does_not_leak_token_material() {
        // Build a secret-shaped string at runtime so no real-looking key is ever
        // committed to source. The helper never receives the token regardless;
        // the message format itself must not contain any secret-shaped chars.
        let prefix = "sk-";
        let body = "FAKEFAKEFAKEFAKEFAKEFAKEFAKEFAKEFAKEFAKE";
        let secret = format!("{prefix}{body}");
        let msg = format_keyring_mismatch(secret.len(), secret.len());
        assert!(!msg.contains(body), "leaked token chars: {msg}");
        assert!(!msg.contains(prefix), "leaked token prefix: {msg}");
        assert!(!msg.contains("preview"), "{msg}");
    }

    /// Migrating a legacy `auth.json` must always rename it to `auth.json.bak`
    /// so the plaintext secret no longer sits at the well-known path — even
    /// when the keyring write fails (headless/CI host).
    #[test]
    fn migrate_legacy_auth_from_renames_plaintext_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        std::fs::write(
            &path,
            r#"{"openai":{"token":"sk-FAKEFAKEFAKEFAKE"},"anthropic":{"token":""}}"#,
        )
        .unwrap();

        migrate_legacy_auth_from(&path).unwrap();

        assert!(!path.exists(), "plaintext auth.json must be moved away");
        let backup = path.with_extension("json.bak");
        assert!(backup.exists(), "backup at auth.json.bak must exist");
    }

    /// No legacy file is a clean no-op (no error, no backup created).
    #[test]
    fn migrate_legacy_auth_from_missing_file_is_noop() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        migrate_legacy_auth_from(&path).unwrap();
        assert!(!path.exists());
        assert!(!path.with_extension("json.bak").exists());
    }
}
