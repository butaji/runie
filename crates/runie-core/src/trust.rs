//! Trust manager — persists per-project trust decisions.
//!
//! First-run flow: when runie starts in a project directory,
//! the TrustManager checks if that path is known. If not,
//! the UI should prompt the user. Untrusted projects default
//! to read-only mode.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustDecision {
    Trusted,
    Untrusted,
}

/// Manages trust decisions for project directories.
/// Persisted to `~/.runie/trust.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TrustManager {
    decisions: HashMap<PathBuf, TrustDecision>,
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
        crate::io::atomic_write::atomic_write(&path, &json)?;
        Ok(())
    }

    /// Get the trust decision for a path (exact match).
    pub fn decision_for(&self, path: &Path) -> Option<TrustDecision> {
        self.decisions.get(path).copied()
    }

    /// Set a trust decision for a path.
    pub fn set(&mut self, path: &Path, decision: TrustDecision) {
        self.decisions.insert(path.to_path_buf(), decision);
    }

    /// Check if a path is trusted (explicitly trusted, or no decision yet).
    pub fn is_trusted(&self, path: &Path) -> bool {
        matches!(self.decision_for(path), Some(TrustDecision::Trusted) | None)
    }

    /// Check if a path is explicitly untrusted.
    pub fn is_untrusted(&self, path: &Path) -> bool {
        matches!(self.decision_for(path), Some(TrustDecision::Untrusted))
    }

    /// Return a copy of all stored decisions.
    pub fn decisions(&self) -> std::collections::HashMap<PathBuf, TrustDecision> {
        self.decisions.clone()
    }

    fn load_path() -> Option<PathBuf> {
        if let Ok(dir) = std::env::var("RUNIE_TEST_CONFIG_DIR") {
            return Some(PathBuf::from(dir).join("trust.json"));
        }
        dirs::config_dir().map(|d| d.join("runie").join("trust.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trust_manager_default_empty() {
        let tm = TrustManager::default();
        assert!(tm.decision_for(Path::new("/foo")).is_none());
    }

    #[test]
    fn trust_sets_decision() {
        let mut tm = TrustManager::default();
        tm.set(Path::new("/foo"), TrustDecision::Trusted);
        assert_eq!(
            tm.decision_for(Path::new("/foo")),
            Some(TrustDecision::Trusted)
        );
    }

    #[test]
    fn untrusted_defaults_untrusted() {
        let mut tm = TrustManager::default();
        tm.set(Path::new("/foo"), TrustDecision::Untrusted);
        assert!(tm.is_untrusted(Path::new("/foo")));
        assert!(!tm.is_trusted(Path::new("/foo")));
    }

    #[test]
    fn unknown_path_is_trusted_by_default() {
        let tm = TrustManager::default();
        assert!(tm.is_trusted(Path::new("/unknown")));
    }

    #[test]
    fn save_load_roundtrip() {
        let mut tm = TrustManager::default();
        tm.set(Path::new("/project/a"), TrustDecision::Trusted);
        tm.set(Path::new("/project/b"), TrustDecision::Untrusted);

        let tmp = std::env::temp_dir().join(format!("runie_trust_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let path = tmp.join("trust.json");

        // Save via direct serialization
        let json = serde_json::to_string_pretty(&tm).unwrap();
        std::fs::write(&path, json).unwrap();

        let loaded: TrustManager =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();

        assert_eq!(
            loaded.decision_for(Path::new("/project/a")),
            Some(TrustDecision::Trusted)
        );
        assert_eq!(
            loaded.decision_for(Path::new("/project/b")),
            Some(TrustDecision::Untrusted)
        );
    }
}
