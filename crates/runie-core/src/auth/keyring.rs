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
    load_all_from_keyring_with(&OsKeyringStore::new())
}

/// Every provider that takes an API key. Must cover all `key:` values in
/// `runie-provider/resources/models/*.yaml` — a provider missing here
/// never gets its token loaded at startup, bouncing the user back to the
/// Login dialog on every launch.
const KEYRING_PROVIDERS: &[&str] = &[
    "anthropic",
    "cohere",
    "deepseek",
    "fireworks",
    "google",
    "groq",
    "minimax",
    "mistral",
    "moonshotai",
    "openai",
    "openrouter",
    "together",
    "xai",
];

/// Load all known provider tokens from the given keyring store.
/// Exposed for tests (with `MockKeyringStore`).
pub fn load_all_from_keyring_with(
    store: &dyn KeyringStore,
) -> anyhow::Result<HashMap<String, AuthToken>> {
    let mut tokens = HashMap::new();
    for provider in KEYRING_PROVIDERS {
        if let Some(token) = store.get(provider)? {
            tokens.insert(
                provider.to_string(),
                AuthToken {
                    provider: provider.to_string(),
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
/// The rename happens only when every token in the file was successfully
/// written to the keyring (or the file held no tokens): on a headless/CI host
/// where the keyring write fails, the file is the last copy of the credential
/// and renaming it away would silently destroy the user's API key. Exposed
/// for tests.
pub fn migrate_legacy_auth_from(path: &std::path::Path) -> anyhow::Result<()> {
    migrate_legacy_auth_with(path, &OsKeyringStore::new())
}

/// Migration with an explicit store — the seam used by tests.
pub fn migrate_legacy_auth_with(
    path: &std::path::Path,
    store: &dyn KeyringStore,
) -> anyhow::Result<()> {
    if !path.exists() {
        return Ok(());
    }

    let json = std::fs::read_to_string(path)?;
    let raw: serde_json::Value = serde_json::from_str(&json).unwrap_or(serde_json::json!({}));

    let mut all_migrated = true;
    if let Some(obj) = raw.as_object() {
        for (provider, val) in obj {
            if let Some(token_str) = val.get("token").and_then(|v| v.as_str()) {
                if !token_str.is_empty() {
                    // Verify the write: some keychain backends (e.g. macOS with a
                    // misconfigured default keychain) report success on set but
                    // return nothing on get. Renaming the file away in that case
                    // would destroy the only retrievable copy of the credential.
                    let migrated = store.set(provider, token_str).is_ok()
                        && store
                            .get(provider)
                            .map(|stored| {
                                stored
                                    .map(|s| s.expose_secret() == token_str)
                                    .unwrap_or(false)
                            })
                            .unwrap_or(false);
                    if !migrated {
                        tracing::warn!(
                            "keyring write for {} could not be verified; keeping auth.json",
                            provider
                        );
                        all_migrated = false;
                    }
                }
            }
        }
    }

    if all_migrated {
        let backup = path.with_extension("json.bak");
        if let Err(e) = std::fs::rename(path, &backup) {
            tracing::debug!("could not rename legacy auth file: {}", e);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::store_trait::MockKeyringStore;
    use secrecy::SecretString;

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

    /// Migrating a legacy `auth.json` renames it to `auth.json.bak` once
    /// every token landed in the keyring.
    #[test]
    fn migrate_legacy_auth_renames_file_after_successful_keyring_write() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        std::fs::write(
            &path,
            r#"{"openai":{"token":"sk-FAKEFAKEFAKEFAKE"},"anthropic":{"token":""}}"#,
        )
        .unwrap();

        let store = MockKeyringStore::new();
        migrate_legacy_auth_with(&path, &store).unwrap();

        assert!(!path.exists(), "plaintext auth.json must be moved away");
        let backup = path.with_extension("json.bak");
        assert!(backup.exists(), "backup at auth.json.bak must exist");
        assert!(
            store.get("openai").unwrap().is_some(),
            "token must land in the keyring"
        );
        assert!(
            store.get("anthropic").unwrap().is_none(),
            "empty tokens are not migrated"
        );
    }

    /// When the keyring write fails (headless/CI host), the plaintext file
    /// must stay in place — it is the last copy of the credential, and
    /// renaming it away would silently destroy the user's API key.
    #[test]
    fn migrate_legacy_auth_keeps_file_when_keyring_write_fails() {
        struct FailingStore;
        impl KeyringStore for FailingStore {
            fn set(&self, _provider: &str, _token: &str) -> anyhow::Result<()> {
                anyhow::bail!("keyring unavailable")
            }
            fn get(&self, _provider: &str) -> anyhow::Result<Option<SecretString>> {
                Ok(None)
            }
            fn delete(&self, _provider: &str) -> anyhow::Result<()> {
                Ok(())
            }
        }

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        std::fs::write(&path, r#"{"minimax":{"token":"sk-FAKEFAKEFAKEFAKE"}}"#).unwrap();

        migrate_legacy_auth_with(&path, &FailingStore).unwrap();

        assert!(
            path.exists(),
            "auth.json must stay when the keyring write failed"
        );
        assert!(
            !path.with_extension("json.bak").exists(),
            "no backup may be created on failed migration"
        );
    }

    /// A keychain that accepts writes but silently loses them (set returns
    /// Ok, get returns None — observed on macOS with a misconfigured default
    /// keychain) must NOT trigger the rename: the plaintext file is the only
    /// retrievable copy of the credential.
    #[test]
    fn migrate_legacy_auth_keeps_file_when_keyring_silently_loses_writes() {
        struct SilentLossStore;
        impl KeyringStore for SilentLossStore {
            fn set(&self, _provider: &str, _token: &str) -> anyhow::Result<()> {
                Ok(()) // write "succeeds"...
            }
            fn get(&self, _provider: &str) -> anyhow::Result<Option<SecretString>> {
                Ok(None) // ...but nothing is retrievable
            }
            fn delete(&self, _provider: &str) -> anyhow::Result<()> {
                Ok(())
            }
        }

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        std::fs::write(&path, r#"{"minimax":{"token":"sk-FAKEFAKEFAKEFAKE"}}"#).unwrap();

        migrate_legacy_auth_with(&path, &SilentLossStore).unwrap();

        assert!(
            path.exists(),
            "auth.json must stay when the keyring write cannot be verified"
        );
        assert!(
            !path.with_extension("json.bak").exists(),
            "no backup may be created when the keyring silently loses writes"
        );
    }

    /// Every key-taking provider (per resources/models/*.yaml) must load —
    /// a provider missing from the list bounces the user to Login on every
    /// launch even though the token is stored.
    #[test]
    fn load_all_from_keyring_with_loads_all_supported_providers() {
        let store = MockKeyringStore::new();
        for provider in [
            "deepseek",
            "fireworks",
            "minimax",
            "moonshotai",
            "openrouter",
            "together",
        ] {
            store.set(provider, "sk-FAKE").unwrap();
        }

        let tokens = load_all_from_keyring_with(&store).unwrap();

        for provider in [
            "deepseek",
            "fireworks",
            "minimax",
            "moonshotai",
            "openrouter",
            "together",
        ] {
            assert!(
                tokens.contains_key(provider),
                "{provider} must load from the keyring"
            );
        }
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
