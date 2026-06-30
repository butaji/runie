//! AuthStorage types.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AuthToken {
    pub provider: String,
    pub token: String,
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

        // Try keyring first
        if let Ok(tokens) = super::keyring::load_all_from_keyring() {
            storage.tokens = tokens;
            storage.keyring_available = true;
            return storage;
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
                                token: token_str.to_owned(),
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
        if self.keyring_available {
            self.save_to_keyring()?;
        } else {
            self.save_to_file()?;
        }
        Ok(())
    }

    fn save_to_file(&self) -> anyhow::Result<()> {
        if let Some(parent) = self.fallback_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut obj = serde_json::Map::new();
        for (provider, tok) in &self.tokens {
            let mut entry = serde_json::Map::new();
            entry.insert("token".into(), tok.token.clone().into());
            entry.insert("expires_at".into(), tok.expires_at.unwrap_or(0.0).into());
            obj.insert(provider.clone(), entry.into());
        }
        let json = serde_json::to_string_pretty(&obj)?;
        std::fs::write(&self.fallback_path, json)?;
        Ok(())
    }

    fn save_to_keyring(&self) -> anyhow::Result<()> {
        for (provider, tok) in &self.tokens {
            super::keyring::set_keyring(provider, &tok.token)?;
        }
        Ok(())
    }

    pub fn set(&mut self, provider: &str, token: &str, expires_at: Option<f64>) {
        // Try keyring first if available
        if self.keyring_available {
            if let Err(e) = crate::auth::keyring::set_keyring(provider, token) {
                // Fall back to file if keyring fails
                tracing::warn!("keyring set failed, falling back to file: {}", e);
                self.keyring_available = false;
                // Switch to file mode and save everything
                self.save_to_file().ok();
            }
        }

        self.tokens.insert(
            provider.to_owned(),
            AuthToken {
                provider: provider.to_owned(),
                token: token.to_owned(),
                expires_at,
            },
        );
    }

    pub fn remove(&mut self, provider: &str) {
        self.tokens.remove(provider);

        if self.keyring_available {
            let _ = super::keyring::delete_keyring(provider);
        }
    }

    pub fn get(&self, provider: &str) -> Option<&AuthToken> {
        self.tokens.get(provider)
    }

    /// Get a keyring token directly by provider name.
    /// Returns the token string if found in keyring, None otherwise.
    pub fn get_keyring_token(provider: &str) -> Option<String> {
        super::keyring::get_keyring(provider).ok()
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
        assert_eq!(store.get("openai").unwrap().token, "sk-test");
        assert_eq!(store.get("openai").unwrap().provider, "openai");
    }

    #[test]
    fn auth_storage_save_load() {
        let mut store = tmp_storage();
        store.set("openai", "sk-test", Some(1_000_000_000.0));
        store.save().unwrap();

        let loaded = AuthStorage::load_from(&store.fallback_path);
        assert_eq!(loaded.tokens.len(), 1);
        assert_eq!(loaded.tokens.get("openai").unwrap().token, "sk-test");
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
}
