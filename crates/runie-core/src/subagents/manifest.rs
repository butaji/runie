//! Manifest — SHA-256 checksums for bundled agent type files.
//!
//! The manifest lives at `resources/agents/manifest.json` and is embedded at
//! compile time.  The manifest structure is deserialized here; checksum
//! validation is performed at build time in `build.rs`.

use serde::Deserialize;

/// Bundled resource manifest.
#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub version: u32,
    #[serde(default)]
    pub description: String,
    pub files: std::collections::HashMap<String, String>,
}

impl Manifest {
    /// Load the embedded manifest JSON.
    pub fn load() -> Self {
        let json = include_str!("../../resources/agents/manifest.json");
        serde_json::from_str(json).expect("manifest.json must be valid JSON")
    }

    /// Number of tracked files.
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Check if the manifest has no files.
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Check whether a specific file's SHA-256 matches the stored hash.
    pub fn check_file(&self, filename: &str, actual_hash: &str) -> bool {
        self.files.get(filename) == Some(&actual_hash.to_lowercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_deserializes() {
        let m = Manifest::load();
        assert_eq!(m.version, 1);
        assert_eq!(m.files.len(), 4);
        assert!(m.files.contains_key("explore.md"));
        assert!(m.files.contains_key("plan.md"));
        assert!(m.files.contains_key("verify.md"));
        assert!(m.files.contains_key("check-work.md"));
    }

    #[test]
    fn check_file_returns_true_on_match() {
        let m = Manifest::load();
        // Use the actual hash from the manifest.
        let hash = m.files.get("explore.md").unwrap();
        assert!(m.check_file("explore.md", hash));
    }

    #[test]
    fn check_file_returns_false_on_mismatch() {
        let m = Manifest::load();
        assert!(!m.check_file(
            "explore.md",
            "0000000000000000000000000000000000000000000000000000000000000000"
        ));
    }
}
