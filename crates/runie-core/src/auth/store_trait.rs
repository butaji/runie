//! Keyring store trait and implementations.
//!
//! Provides a `KeyringStore` trait for testable credential storage,
//! with `OsKeyringStore` (production) and `MockKeyringStore` (testing) backends.

use std::collections::HashMap;

use parking_lot::RwLock;
use secrecy::SecretString;

// ── Trait ─────────────────────────────────────────────────────────────────────

/// Trait for keyring storage backends.
///
/// Implementors must be thread-safe (`Send + Sync`) so they can be shared
/// across actor boundaries without interior mutability concerns.
pub trait KeyringStore: Send + Sync {
    /// Store a token for the given provider.
    fn set(&self, provider: &str, token: &str) -> anyhow::Result<()>;

    /// Retrieve a token for the given provider.
    /// Returns `Ok(Some(token))` if found, `Ok(None)` if not present.
    /// Returns `SecretString` to prevent accidental exposure in logs.
    fn get(&self, provider: &str) -> anyhow::Result<Option<SecretString>>;

    /// Delete the token for the given provider.
    fn delete(&self, provider: &str) -> anyhow::Result<()>;
}

// ── OS Keyring Backend ─────────────────────────────────────────────────────────

/// OS keyring backend using the `keyring` crate.
/// All entries use the `"runie"` service name with `"provider:{name}"` accounts.
pub struct OsKeyringStore;

impl OsKeyringStore {
    pub fn new() -> Self {
        Self
    }
}

impl Default for OsKeyringStore {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyringStore for OsKeyringStore {
    fn set(&self, provider: &str, token: &str) -> anyhow::Result<()> {
        let entry = keyring::Entry::new("runie", &format!("provider:{provider}"))?;
        entry.set_password(token)?;
        Ok(())
    }

    fn get(&self, provider: &str) -> anyhow::Result<Option<SecretString>> {
        let entry = keyring::Entry::new("runie", &format!("provider:{provider}"))?;
        match entry.get_password() {
            Ok(token) => Ok(Some(SecretString::from(token))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(anyhow::anyhow!("keyring error: {e}")),
        }
    }

    fn delete(&self, provider: &str) -> anyhow::Result<()> {
        let entry = keyring::Entry::new("runie", &format!("provider:{provider}"))?;
        entry
            .delete_credential()
            .map_err(|e| anyhow::anyhow!("keyring error: {e}"))
    }
}

// ── In-Memory Mock Backend ─────────────────────────────────────────────────────

/// In-memory mock keyring store for testing without OS keychain access.
/// Uses a `RwLock` so reads are non-blocking and writes are exclusive.
pub struct MockKeyringStore {
    entries: RwLock<HashMap<String, String>>,
}

impl MockKeyringStore {
    pub fn new() -> Self {
        Self { entries: RwLock::new(HashMap::new()) }
    }
}

impl Default for MockKeyringStore {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyringStore for MockKeyringStore {
    fn set(&self, provider: &str, token: &str) -> anyhow::Result<()> {
        self.entries.write().insert(provider.to_owned(), token.to_owned());
        Ok(())
    }

    fn get(&self, provider: &str) -> anyhow::Result<Option<SecretString>> {
        Ok(self.entries.read().get(provider).map(|s| SecretString::from(s.clone())))
    }

    fn delete(&self, provider: &str) -> anyhow::Result<()> {
        self.entries.write().remove(provider);
        Ok(())
    }
}

// ── Unit Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use secrecy::ExposeSecret;
    use super::*;

    // OS keyring tests interact with the real macOS/Linux keychain.
    // Run with: cargo test --ignored -- os_keyring
    // These may fail in CI if keyring access is restricted.

    #[test]
    #[ignore]
    fn os_keyring_set_and_get() {
        let store = OsKeyringStore::new();
        let provider = format!("test_os_{}", std::process::id());
        store.set(&provider, "secret-token").unwrap();
        let result = store.get(&provider).unwrap();
        assert_eq!(result.as_ref().map(|s| s.expose_secret().as_str()), Some("secret-token"));
        store.delete(&provider).unwrap();
    }

    #[test]
    #[ignore]
    fn os_keyring_get_nonexistent() {
        let store = OsKeyringStore::new();
        let result = store.get("nonexistent_provider_xyz_abc").unwrap();
        assert!(result.is_none());
    }

    #[test]
    #[ignore]
    fn os_keyring_delete_nonexistent() {
        let store = OsKeyringStore::new();
        let result = store.delete("nonexistent_provider_xyz_abc");
        assert!(result.is_ok());
    }

    #[test]
    fn mock_keyring_set_and_get() {
        let store = MockKeyringStore::new();
        store.set("openai", "sk-test").unwrap();
        assert_eq!(store.get("openai").unwrap().as_ref().map(|s| s.expose_secret().as_str()), Some("sk-test"));
        assert!(store.get("anthropic").unwrap().is_none());
    }

    #[test]
    fn mock_keyring_overwrite() {
        let store = MockKeyringStore::new();
        store.set("openai", "sk-old").unwrap();
        store.set("openai", "sk-new").unwrap();
        assert_eq!(store.get("openai").unwrap().as_ref().map(|s| s.expose_secret().as_str()), Some("sk-new"));
    }

    #[test]
    fn mock_keyring_delete() {
        let store = MockKeyringStore::new();
        store.set("openai", "sk-test").unwrap();
        store.delete("openai").unwrap();
        assert!(store.get("openai").unwrap().is_none());
    }

    #[test]
    fn mock_keyring_delete_nonexistent() {
        let store = MockKeyringStore::new();
        // Deleting a non-existent key should succeed
        let result = store.delete("never_existed");
        assert!(result.is_ok());
        assert!(store.get("never_existed").unwrap().is_none());
    }

    #[test]
    fn mock_keyring_shared_across_threads() {
        let store = Arc::new(MockKeyringStore::new());
        let store_clone = Arc::clone(&store);

        store.set("openai", "sk-thread").unwrap();

        // Verify the shared store has the value
        assert_eq!(store_clone.get("openai").unwrap().as_ref().map(|s| s.expose_secret().as_str()), Some("sk-thread"));
    }
}
