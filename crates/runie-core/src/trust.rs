//! Trust manager — persists per-project trust decisions.
//!
//! First-run flow: when runie starts in a project directory,
//! the TrustManager checks if that path is known. If not,
//! the UI should prompt the user. Untrusted projects default
//! to read-only mode.

use camino::Utf8PathBuf;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustDecision {
    Trusted,
    Untrusted,
}

/// Manages trust decisions for project directories.
/// Persisted to `~/.runie/trust.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TrustManager {
    decisions: IndexMap<Utf8PathBuf, TrustDecision>,
}

impl TrustManager {
    /// Load trust decisions from disk, or return empty manager.
    pub fn load() -> Self {
        match Self::load_path() {
            Some(path) => match std::fs::read_to_string(&path) {
                Ok(json) => serde_json::from_str(&json).unwrap_or_default(),
                Err(_) => Self::default(),
            },
            None => Self::default(),
        }
    }

    /// Save trust decisions to disk atomically with restricted permissions.
    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::load_path().ok_or_else(|| anyhow::anyhow!("No config directory"))?;
        let json = serde_json::to_string_pretty(self)?;
        crate::io::atomic_write::atomic_write(path.as_ref(), &json)?;
        Ok(())
    }

    /// Get the trust decision for a path (exact match).
    pub fn decision_for(&self, path: &Utf8PathBuf) -> Option<TrustDecision> {
        self.decisions.get(path).copied()
    }

    /// Set a trust decision for a path.
    pub fn set(&mut self, path: &Utf8PathBuf, decision: TrustDecision) {
        self.decisions.insert(path.clone(), decision);
    }

    /// Check if a path is trusted (explicitly trusted, or no decision yet).
    pub fn is_trusted(&self, path: &Utf8PathBuf) -> bool {
        matches!(self.decision_for(path), Some(TrustDecision::Trusted) | None)
    }

    /// Check if a path is explicitly untrusted.
    pub fn is_untrusted(&self, path: &Utf8PathBuf) -> bool {
        matches!(self.decision_for(path), Some(TrustDecision::Untrusted))
    }

    /// Return a copy of all stored decisions.
    pub fn decisions(&self) -> IndexMap<Utf8PathBuf, TrustDecision> {
        self.decisions.clone()
    }

    fn load_path() -> Option<Utf8PathBuf> {
        if let Ok(dir) = std::env::var("RUNIE_TEST_CONFIG_DIR") {
            return Some(Utf8PathBuf::from(dir).join("trust.json"));
        }
        dirs::config_dir()
            .and_then(|d| Utf8PathBuf::from_path_buf(d).ok())
            .map(|d| d.join("runie").join("trust.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // `load_path()` reads the process-global `RUNIE_TEST_CONFIG_DIR` env var, so
    // tests that override it must not run concurrently — otherwise one test's
    // set/remove races with another's save() and the file lands in the wrong
    // directory. Serialize the env-touching tests behind this lock.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn trust_manager_default_empty() {
        let tm = TrustManager::default();
        assert!(tm.decision_for(&Utf8PathBuf::from("/foo")).is_none());
    }

    #[test]
    fn trust_sets_decision() {
        let mut tm = TrustManager::default();
        tm.set(&Utf8PathBuf::from("/foo"), TrustDecision::Trusted);
        assert_eq!(
            tm.decision_for(&Utf8PathBuf::from("/foo")),
            Some(TrustDecision::Trusted)
        );
    }

    #[test]
    fn untrusted_defaults_untrusted() {
        let mut tm = TrustManager::default();
        tm.set(&Utf8PathBuf::from("/foo"), TrustDecision::Untrusted);
        assert!(tm.is_untrusted(&Utf8PathBuf::from("/foo")));
        assert!(!tm.is_trusted(&Utf8PathBuf::from("/foo")));
    }

    #[test]
    fn unknown_path_is_trusted_by_default() {
        let tm = TrustManager::default();
        assert!(tm.is_trusted(&Utf8PathBuf::from("/unknown")));
    }

    #[test]
    fn save_load_roundtrip() {
        let mut tm = TrustManager::default();
        tm.set(&Utf8PathBuf::from("/project/a"), TrustDecision::Trusted);
        tm.set(&Utf8PathBuf::from("/project/b"), TrustDecision::Untrusted);

        let tmp = std::env::temp_dir().join(format!("runie_trust_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let path = tmp.join("trust.json");

        // Serialize with other tests that override RUNIE_TEST_CONFIG_DIR.
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // Set env so save() uses our temp directory
        std::env::set_var("RUNIE_TEST_CONFIG_DIR", &tmp);

        // Save via atomic write path (tm.save())
        tm.save().unwrap();

        std::env::remove_var("RUNIE_TEST_CONFIG_DIR");
        drop(_guard);

        // Verify file was created at expected path
        assert!(path.exists(), "trust.json should exist after save");

        let loaded: TrustManager = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();

        assert_eq!(
            loaded.decision_for(&Utf8PathBuf::from("/project/a")),
            Some(TrustDecision::Trusted)
        );
        assert_eq!(
            loaded.decision_for(&Utf8PathBuf::from("/project/b")),
            Some(TrustDecision::Untrusted)
        );
    }

    #[cfg(unix)]
    #[test]
    fn save_sets_restricted_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let mut tm = TrustManager::default();
        tm.set(&Utf8PathBuf::from("/project/a"), TrustDecision::Trusted);

        let tmp = std::env::temp_dir().join(format!("runie_trust_perms_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let path = tmp.join("trust.json");

        // Serialize with other tests that override RUNIE_TEST_CONFIG_DIR.
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // Set env so save() uses our temp directory
        std::env::set_var("RUNIE_TEST_CONFIG_DIR", &tmp);

        tm.save().unwrap();

        std::env::remove_var("RUNIE_TEST_CONFIG_DIR");
        drop(_guard);

        // Verify 0o600 permissions (user read/write only)
        let perms = std::fs::metadata(&path).unwrap().permissions();
        let mode = perms.mode();
        assert_eq!(
            mode & 0o777,
            0o600,
            "File should have 0o600 permissions, got {:o}",
            mode
        );
    }
}
