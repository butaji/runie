//! OAuth / API-key authentication storage.
//!
//! Tokens are persisted in `~/.runie/auth.json` with a lightweight
//! XOR cipher (key derived from machine identity).  This is
//! obfuscation-grade encryption — sufficient to prevent casual
//! inspection of the file, not a replacement for a hardware security
//! module.

use std::collections::HashMap;
use std::path::PathBuf;

const FILENAME: &str = "auth.json";

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AuthToken {
    pub provider: String,
    pub token: String,
    pub expires_at: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AuthStorage {
    pub tokens: HashMap<String, AuthToken>,
    pub path: PathBuf,
}

impl AuthStorage {
    /// Load storage from the default path (`~/.runie/auth.json`).
    pub fn load() -> Self {
        let path = default_auth_path().unwrap_or_else(|| PathBuf::from("/tmp/runie_auth.json"));
        Self::load_from(&path)
    }

    /// Load from an explicit path (useful in tests).
    pub fn load_from(path: &PathBuf) -> Self {
        if !path.exists() {
            return Self {
                tokens: HashMap::new(),
                path: path.clone(),
            };
        }
        match std::fs::read_to_string(path) {
            Ok(json) => Self::from_json(&json, path.clone()),
            Err(_) => Self {
                tokens: HashMap::new(),
                path: path.clone(),
            },
        }
    }

    fn from_json(json: &str, path: PathBuf) -> Self {
        let raw: serde_json::Value = serde_json::from_str(json).unwrap_or(serde_json::json!({}));
        let mut tokens = HashMap::new();
        if let Some(obj) = raw.as_object() {
            for (provider, val) in obj {
                if let Some(enc) = val.get("token").and_then(|v| v.as_str()) {
                    let exp = val.get("expires_at").and_then(|v| v.as_f64());
                    if let Some(token) = decrypt_token(enc) {
                        tokens.insert(
                            provider.clone(),
                            AuthToken {
                                provider: provider.clone(),
                                token,
                                expires_at: exp.filter(|e| *e > 0.0),
                            },
                        );
                    }
                }
            }
        }
        Self { tokens, path }
    }

    /// Persist tokens to disk.
    pub fn save(&self) -> anyhow::Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut obj = serde_json::Map::new();
        for (provider, tok) in &self.tokens {
            let mut entry = serde_json::Map::new();
            entry.insert("token".into(), encrypt_token(&tok.token).into());
            entry.insert("expires_at".into(), tok.expires_at.unwrap_or(0.0).into());
            obj.insert(provider.clone(), entry.into());
        }
        let json = serde_json::to_string_pretty(&obj)?;
        std::fs::write(&self.path, json)?;
        Ok(())
    }

    pub fn set(&mut self, provider: &str, token: &str, expires_at: Option<f64>) {
        self.tokens.insert(
            provider.to_string(),
            AuthToken {
                provider: provider.to_string(),
                token: token.to_string(),
                expires_at,
            },
        );
    }

    pub fn remove(&mut self, provider: &str) {
        self.tokens.remove(provider);
    }

    pub fn get(&self, provider: &str) -> Option<&AuthToken> {
        self.tokens.get(provider)
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
}

fn default_auth_path() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("runie").join(FILENAME))
}

/// Simple XOR cipher — key is derived from machine identity.
fn encrypt_token(token: &str) -> String {
    let key = machine_key();
    let mut out = Vec::with_capacity(token.len());
    for (i, b) in token.bytes().enumerate() {
        out.push(b ^ key[i % key.len()]);
    }
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    STANDARD.encode(&out)
}

fn decrypt_token(encrypted: &str) -> Option<String> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    let bytes = STANDARD.decode(encrypted).ok()?;
    let key = machine_key();
    let mut out = Vec::with_capacity(bytes.len());
    for (i, b) in bytes.iter().enumerate() {
        out.push(b ^ key[i % key.len()]);
    }
    String::from_utf8(out).ok()
}

fn machine_key() -> Vec<u8> {
    let mut parts = Vec::new();
    if let Ok(hostname) = std::process::Command::new("hostname").output() {
        parts.push(hostname.stdout);
    }
    if let Some(home) = dirs::home_dir() {
        parts.push(home.to_string_lossy().as_bytes().to_vec());
    }
    if parts.is_empty() {
        return b"runie-default-key".to_vec();
    }
    let mut key = Vec::new();
    for p in parts {
        key.extend_from_slice(&p);
    }
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_storage() -> AuthStorage {
        let n = std::sync::atomic::AtomicU64::new(0);
        let id = n.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let path =
            std::env::temp_dir().join(format!("runie_auth_test_{}_{}", std::process::id(), id));
        AuthStorage {
            tokens: HashMap::new(),
            path,
        }
    }

    #[test]
    fn auth_storage_save_load() {
        let mut store = tmp_storage();
        store.set("openai", "sk-test", Some(1_000_000_000.0));
        store.save().unwrap();

        let loaded = AuthStorage::load_from(&store.path);
        assert_eq!(loaded.tokens.len(), 1);
        let tok = loaded.tokens.get("openai").unwrap();
        assert_eq!(tok.token, "sk-test");
        assert_eq!(tok.provider, "openai");
        assert!(tok.expires_at.unwrap() > 0.0);
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
    fn encrypt_decrypt_roundtrip() {
        let original = "hello-world-token-123";
        let enc = encrypt_token(original);
        let dec = decrypt_token(&enc).unwrap();
        assert_eq!(dec, original);
    }
}
