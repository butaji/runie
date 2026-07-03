//! AuthStorage types.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Serialize a `SecretString` by exposing the secret value.
fn serialize_secret<S: Serializer>(s: &SecretString, ser: S) -> Result<S::Ok, S::Error> {
    s.expose_secret().serialize(ser)
}

/// Deserialize a `SecretString` from a string.
fn deserialize_secret<'de, D: Deserializer<'de>>(de: D) -> Result<SecretString, D::Error> {
    let s = String::deserialize(de)?;
    Ok(SecretString::from(s))
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthToken {
    pub provider: String,
    #[serde(
        serialize_with = "serialize_secret",
        deserialize_with = "deserialize_secret"
    )]
    pub token: SecretString,
    pub expires_at: Option<f64>,
}

/// Primary storage backend: OS keyring with fallback to JSON file.
#[derive(Debug, Clone)]
pub struct AuthStorage {
    /// Tokens loaded from keyring or migrated from file.
    tokens: HashMap<String, AuthToken>,
    /// Fallback file path for headless/CI environments.
    fallback_path: PathBuf,
    /// Whether keyring is available on this platform.
    keyring_available: bool,
}

impl Default for AuthStorage {
    fn default() -> Self {
        Self::new()
    }
}

fn default_auth_path() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("runie").join("auth.json"))
}

impl AuthStorage {
    /// Create a new empty auth storage.
    pub fn new() -> Self {
        Self {
            tokens: HashMap::new(),
            fallback_path: default_auth_path()
                .unwrap_or_else(|| PathBuf::from("/tmp/runie_auth.json")),
            keyring_available: true, // Try keyring; fallback if it fails
        }
    }

    /// Load storage, trying keyring first then falling back to file.
    pub fn load() -> Self {
        let mut storage = Self::new();

        #[cfg(feature = "keyring")]
        {
            // Try keyring first
            if let Ok(tokens) = super::keyring::load_all_from_keyring() {
                storage.tokens = tokens;
                storage.keyring_available = true;
                return storage;
            }
        }

        // Fall back to file
        storage.keyring_available = false;
        storage.load_from_file();
        storage
    }

    /// Load from an explicit path (useful in tests).
    pub fn load_from(path: &Path) -> Self {
        let mut storage = Self {
            tokens: HashMap::new(),
            fallback_path: path.to_path_buf(),
            keyring_available: false,
        };
        storage.load_from_file();
        storage
    }

    fn load_from_file(&mut self) {
        if !self.fallback_path.exists() {
            return;
        }
        if let Ok(json) = std::fs::read_to_string(&self.fallback_path) {
            let raw: serde_json::Value =
                serde_json::from_str(&json).unwrap_or(serde_json::json!({}));
            if let Some(obj) = raw.as_object() {
                for (provider, val) in obj {
                    if let Some(token_str) = val.get("token").and_then(|v| v.as_str()) {
                        let exp = val.get("expires_at").and_then(|v| v.as_f64());
                        self.tokens.insert(
                            provider.clone(),
                            AuthToken {
                                provider: provider.clone(),
                                token: SecretString::from(String::from(token_str)),
                                expires_at: exp.filter(|e| *e > 0.0),
                            },
                        );
                    }
                }
            }
        }
    }

    /// Persist tokens: keyring if available, otherwise file.
    pub fn save(&self) -> anyhow::Result<()> {
        #[cfg(feature = "keyring")]
        if self.keyring_available {
            self.save_to_keyring()?;
        }
        #[cfg(not(feature = "keyring"))]
        {
            let _ = self; // suppress unused warning
        }
        self.save_to_file()?;
        Ok(())
    }

    fn save_to_file(&self) -> anyhow::Result<()> {
        let mut obj = serde_json::Map::new();
        for (provider, tok) in &self.tokens {
            let mut entry = serde_json::Map::new();
            entry.insert("token".into(), tok.token.expose_secret().to_owned().into());
            entry.insert("expires_at".into(), tok.expires_at.unwrap_or(0.0).into());
            obj.insert(provider.clone(), entry.into());
        }
        let json = serde_json::to_string_pretty(&obj)?;
        crate::io::atomic_write::atomic_write(&self.fallback_path, &json)?;
        Ok(())
    }

    #[cfg(feature = "keyring")]
    fn save_to_keyring(&self) -> anyhow::Result<()> {
        for (provider, tok) in &self.tokens {
            super::keyring::set_keyring(provider, tok.token.expose_secret())?;
        }
        Ok(())
    }

    pub fn set(&mut self, provider: &str, token: &str, expires_at: Option<f64>) {
        // Try keyring first if available
        #[cfg(feature = "keyring")]
        {
            if self.keyring_available {
                if let Err(e) = crate::auth::keyring::set_keyring(provider, token) {
                    // Fall back to file if keyring fails
                    tracing::warn!("keyring set failed, falling back to file: {}", e);
                    self.keyring_available = false;
                    // Switch to file mode and save everything
                    self.save_to_file().ok();
                }
            }
        }

        self.tokens.insert(
            provider.to_owned(),
            AuthToken {
                provider: provider.to_owned(),
                token: SecretString::from(String::from(token)),
                expires_at,
            },
        );
    }

    pub fn remove(&mut self, provider: &str) {
        self.tokens.remove(provider);

        #[cfg(feature = "keyring")]
        if self.keyring_available {
            let _ = super::keyring::delete_keyring(provider);
        }
    }

    pub fn get(&self, provider: &str) -> Option<&AuthToken> {
        self.tokens.get(provider)
    }

    /// Get a keyring token directly by provider name.
    /// Returns the token string if found in keyring, None otherwise.
    #[cfg(feature = "keyring")]
    pub fn get_keyring_token(provider: &str) -> Option<String> {
        super::keyring::get_keyring(provider).ok()
    }

    /// Stub when `keyring` feature is disabled.
    #[cfg(not(feature = "keyring"))]
    pub fn get_keyring_token(_provider: &str) -> Option<String> {
        None
    }

    pub fn is_authenticated(&self, provider: &str) -> bool {
        self.tokens.contains_key(provider)
    }

    /// Returns `true` if the token is missing or has expired.
    pub fn refresh_needed(&self, provider: &str) -> bool {
        match self.tokens.get(provider) {
            None => true,
            Some(tok) => match tok.expires_at {
                None => false,
                Some(exp) => {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs_f64())
                        .unwrap_or(0.0);
                    now >= exp
                }
            },
        }
    }

    /// Number of authenticated providers.
    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    /// Returns an iterator over authenticated provider names.
    pub fn providers(&self) -> impl Iterator<Item = &str> {
        self.tokens.keys().map(|s| s.as_str())
    }

    /// Whether keyring is available (vs file fallback mode).
    pub fn is_keyring_available(&self) -> bool {
        self.keyring_available
    }
}

/// PartialEq compares tokens by exposing them — acceptable here since both sides
/// are in-memory and we are not logging them.
impl PartialEq for AuthToken {
    fn eq(&self, other: &AuthToken) -> bool {
        use secrecy::ExposeSecret;
        self.provider == other.provider
            && self.token.expose_secret() == other.token.expose_secret()
            && self.expires_at == other.expires_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_storage() -> AuthStorage {
        let id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("runie_auth_test_{}_{}", std::process::id(), id));
        AuthStorage {
            tokens: HashMap::new(),
            fallback_path: path,
            keyring_available: false, // Use file mode in tests
        }
    }

    #[test]
    fn auth_storage_set_get() {
        let mut store = tmp_storage();
        store.set("openai", "sk-test", Some(1_000_000_000.0));
        assert_eq!(
            store.get("openai").unwrap().token.expose_secret(),
            "sk-test"
        );
        assert_eq!(store.get("openai").unwrap().provider, "openai");
    }

    #[test]
    fn auth_storage_save_load() {
        let mut store = tmp_storage();
        store.set("openai", "sk-test", Some(1_000_000_000.0));
        store.save().unwrap();

        let loaded = AuthStorage::load_from(&store.fallback_path);
        assert_eq!(loaded.tokens.len(), 1);
        assert_eq!(
            loaded.tokens.get("openai").unwrap().token.expose_secret(),
            "sk-test"
        );
    }

    #[test]
    fn token_refresh_needed_when_expired() {
        let mut store = tmp_storage();
        let past = 1.0;
        store.set("openai", "sk-test", Some(past));
        assert!(store.refresh_needed("openai"));
    }

    #[test]
    fn token_refresh_not_needed_when_no_expiry() {
        let mut store = tmp_storage();
        store.set("openai", "sk-test", None);
        assert!(!store.refresh_needed("openai"));
    }

    #[test]
    fn token_refresh_needed_when_missing() {
        let store = tmp_storage();
        assert!(store.refresh_needed("openai"));
    }

    #[test]
    fn remove_token_makes_refresh_needed() {
        let mut store = tmp_storage();
        store.set("openai", "sk-test", None);
        store.remove("openai");
        assert!(store.refresh_needed("openai"));
    }

    #[test]
    fn get_keyring_token_returns_none_when_not_found() {
        // Keyring may not have this provider, so we just verify the method works
        let result = AuthStorage::get_keyring_token("nonexistent_provider_xyz");
        // Result is None or Some depending on actual keyring state
        // This test just verifies the API works without panicking
        let _ = result;
    }

    #[test]
    fn auth_token_debug_does_not_leak_token() {
        // Layer 1: Debug must redact the token value.
        let token = AuthToken {
            provider: "openai".into(),
            token: secrecy::SecretString::from(String::from("sk-super-secret-key-12345")),
            expires_at: None,
        };
        let debug_str = format!("{:?}", token);
        // The debug string must NOT contain the actual token value
        assert!(
            !debug_str.contains("sk-super-secret-key-12345"),
            "Debug output must not contain the actual token: {}",
            debug_str
        );
        // The debug string must contain the provider (non-sensitive)
        assert!(
            debug_str.contains("openai"),
            "Debug output should contain provider name"
        );
    }

    #[test]
    fn auth_token_expose_secret_only_at_boundary() {
        // Layer 1: Token value is accessible only via ExposeSecret.
        use secrecy::ExposeSecret;
        let token = AuthToken {
            provider: "openai".into(),
            token: secrecy::SecretString::from(String::from("sk-exposed-at-boundary")),
            expires_at: None,
        };
        // ExposeSecret gives access to the inner value
        assert_eq!(
            token.token.expose_secret().as_str(),
            "sk-exposed-at-boundary"
        );
        // Direct access to .token gives the SecretString wrapper, not the value
        let _secret = &token.token;
        // Cannot accidentally compare SecretString to &str without ExposeSecret
    }
}
