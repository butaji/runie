//! Session index for quick access to session metadata.
//!
//! Maintains `sessions.json` in `data_dir/runie/` with metadata per session:
//! - id, display_name, created_at, updated_at, message_count, summary
//! - is_starred, is_system

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Metadata for a single session in the index.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionMetadata {
    pub id: String,
    pub display_name: String,
    pub created_at: f64,
    pub updated_at: f64,
    pub message_count: usize,
    pub summary: Option<String>,
    #[serde(default)]
    pub is_starred: bool,
    #[serde(default)]
    pub is_system: bool,
}

/// Session index — maps session IDs to metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionIndex {
    #[serde(default)]
    pub sessions: Vec<SessionMetadata>,
}

impl SessionIndex {
    /// Path to the index file.
    pub fn path(data_dir: &Path) -> PathBuf {
        data_dir.join("runie").join("sessions.json")
    }

    /// Load index from disk.
    pub fn load(data_dir: &Path) -> anyhow::Result<Self> {
        let path = Self::path(data_dir);
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(&path)?;
        let index: SessionIndex = serde_json::from_str(&content)?;
        Ok(index)
    }

    /// Save index to disk atomically.
    pub fn save(&self, data_dir: &Path) -> anyhow::Result<()> {
        let path = Self::path(data_dir);
        let temp_path = path.with_extension("tmp");
        let dir = path.parent().ok_or_else(|| {
            anyhow::anyhow!("session index path has no parent directory: {}", path.display())
        })?;

        fs::create_dir_all(dir)?;
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&temp_path, content)?;
        fs::rename(&temp_path, &path)?;
        Ok(())
    }

    /// Get metadata for a session by ID.
    pub fn get(&self, id: &str) -> Option<&SessionMetadata> {
        self.sessions.iter().find(|s| s.id == id)
    }

    /// Get mutable metadata for a session by ID.
    pub fn get_mut(&mut self, id: &str) -> Option<&mut SessionMetadata> {
        self.sessions.iter_mut().find(|s| s.id == id)
    }

    /// Add or update a session's metadata.
    pub fn upsert(&mut self, meta: SessionMetadata) {
        if let Some(existing) = self.sessions.iter_mut().find(|s| s.id == meta.id) {
            *existing = meta;
        } else {
            self.sessions.push(meta);
        }
    }

    /// Remove a session from the index.
    pub fn remove(&mut self, id: &str) {
        self.sessions.retain(|s| s.id != id);
    }

    /// Get starred sessions (pinned to top).
    pub fn starred(&self) -> Vec<&SessionMetadata> {
        self.sessions.iter().filter(|s| s.is_starred).collect()
    }

    /// Get system sessions (pinned above regular).
    pub fn system_sessions(&self) -> Vec<&SessionMetadata> {
        self.sessions.iter().filter(|s| s.is_system).collect()
    }

    /// Get regular sessions sorted by updated_at descending.
    pub fn regular_sessions(&self) -> Vec<&SessionMetadata> {
        let mut sessions: Vec<_> = self
            .sessions
            .iter()
            .filter(|s| !s.is_starred && !s.is_system)
            .collect();
        sessions.sort_by(|a, b| {
            b.updated_at
                .partial_cmp(&a.updated_at)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sessions
    }

    /// Search sessions by name or summary (simple substring match).
    pub fn search(&self, query: &str) -> Vec<&SessionMetadata> {
        let query_lower = query.to_lowercase();
        self.sessions
            .iter()
            .filter(|s| {
                s.display_name.to_lowercase().contains(&query_lower)
                    || s.summary
                        .as_ref()
                        .is_some_and(|sum| sum.to_lowercase().contains(&query_lower))
            })
            .collect()
    }

    /// Toggle star status for a session.
    pub fn toggle_star(&mut self, id: &str) -> bool {
        if let Some(session) = self.get_mut(id) {
            session.is_starred = !session.is_starred;
            return session.is_starred;
        }
        false
    }

    /// Rename a session.
    pub fn rename(&mut self, id: &str, new_name: &str) -> bool {
        if let Some(session) = self.get_mut(id) {
            session.display_name = new_name.to_owned();
            session.updated_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs_f64())
                .unwrap_or(0.0);
            return true;
        }
        false
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn test_data_dir() -> PathBuf {
        tempdir().unwrap().path().to_path_buf()
    }

    fn make_meta(id: &str, name: &str, ts: f64) -> SessionMetadata {
        SessionMetadata {
            id: id.into(),
            display_name: name.into(),
            created_at: ts,
            updated_at: ts,
            message_count: 5,
            summary: Some(format!("Summary of {}", name)),
            is_starred: false,
            is_system: false,
        }
    }

    #[test]
    fn session_index_round_trips() {
        let dir = test_data_dir();
        let mut index = SessionIndex::default();

        index.upsert(make_meta("s1", "Session One", 1000.0));
        index.upsert(make_meta("s2", "Session Two", 2000.0));
        index.save(&dir).unwrap();

        let loaded = SessionIndex::load(&dir).unwrap();
        assert_eq!(loaded.sessions.len(), 2);
        assert_eq!(loaded.get("s1").unwrap().display_name, "Session One");
        assert_eq!(
            loaded.get("s2").unwrap().summary.as_deref(),
            Some("Summary of Session Two")
        );
    }

    #[test]
    fn upsert_updates_existing() {
        let mut index = SessionIndex::default();
        index.upsert(make_meta("s1", "Original", 1000.0));
        index.upsert(SessionMetadata {
            id: "s1".into(),
            display_name: "Updated".into(),
            created_at: 1000.0,
            updated_at: 2000.0,
            message_count: 10,
            summary: Some("New summary".into()),
            is_starred: false,
            is_system: false,
        });

        assert_eq!(index.sessions.len(), 1);
        assert_eq!(index.get("s1").unwrap().display_name, "Updated");
        assert_eq!(index.get("s1").unwrap().message_count, 10);
    }

    #[test]
    fn remove_session() {
        let mut index = SessionIndex::default();
        index.upsert(make_meta("s1", "One", 1000.0));
        index.upsert(make_meta("s2", "Two", 2000.0));

        index.remove("s1");
        assert!(index.get("s1").is_none());
        assert!(index.get("s2").is_some());
    }

    #[test]
    fn starred_sessions_sorted_to_top() {
        let mut index = SessionIndex::default();
        index.upsert(SessionMetadata {
            id: "s1".into(),
            display_name: "Not Starred".into(),
            created_at: 1000.0,
            updated_at: 1000.0,
            message_count: 1,
            summary: None,
            is_starred: false,
            is_system: false,
        });
        index.upsert(SessionMetadata {
            id: "s2".into(),
            display_name: "Starred Session".into(),
            created_at: 2000.0,
            updated_at: 2000.0,
            message_count: 2,
            summary: None,
            is_starred: true,
            is_system: false,
        });

        let starred = index.starred();
        assert_eq!(starred.len(), 1);
        assert_eq!(starred[0].id, "s2");
    }

    #[test]
    fn system_sessions_pinned_above_regular() {
        let mut index = SessionIndex::default();
        index.upsert(SessionMetadata {
            id: "s1".into(),
            display_name: "Regular".into(),
            created_at: 1000.0,
            updated_at: 1000.0,
            message_count: 1,
            summary: None,
            is_starred: false,
            is_system: false,
        });
        index.upsert(SessionMetadata {
            id: "s2".into(),
            display_name: "Scheduled Tasks".into(),
            created_at: 500.0,
            updated_at: 500.0,
            message_count: 3,
            summary: None,
            is_starred: false,
            is_system: true,
        });

        let systems = index.system_sessions();
        assert_eq!(systems.len(), 1);
        assert_eq!(systems[0].id, "s2");
    }

    #[test]
    fn search_finds_by_name_and_summary() {
        let mut index = SessionIndex::default();
        index.upsert(SessionMetadata {
            id: "s1".into(),
            display_name: "Python Research".into(),
            created_at: 1000.0,
            updated_at: 1000.0,
            message_count: 5,
            summary: Some("Discussed async/await patterns".into()),
            is_starred: false,
            is_system: false,
        });
        index.upsert(SessionMetadata {
            id: "s2".into(),
            display_name: "Rust Debugging".into(),
            created_at: 2000.0,
            updated_at: 2000.0,
            message_count: 3,
            summary: Some("Fixed borrow checker issues".into()),
            is_starred: false,
            is_system: false,
        });

        let rust_results = index.search("rust");
        assert_eq!(rust_results.len(), 1);
        assert_eq!(rust_results[0].id, "s2");

        let async_results = index.search("async");
        assert_eq!(async_results.len(), 1);
        assert_eq!(async_results[0].id, "s1");
    }

    #[test]
    fn toggle_star() {
        let mut index = SessionIndex::default();
        index.upsert(make_meta("s1", "Test", 1000.0));

        assert!(!index.get("s1").unwrap().is_starred);
        let new_state = index.toggle_star("s1");
        assert!(new_state);
        assert!(index.get("s1").unwrap().is_starred);

        let reverted = index.toggle_star("s1");
        assert!(!reverted);
        assert!(!index.get("s1").unwrap().is_starred);
    }

    #[test]
    fn rename_updates_timestamp() {
        let mut index = SessionIndex::default();
        index.upsert(make_meta("s1", "Old Name", 1000.0));

        let original_updated = index.get("s1").unwrap().updated_at;
        index.rename("s1", "New Name");

        assert_eq!(index.get("s1").unwrap().display_name, "New Name");
        assert!(index.get("s1").unwrap().updated_at >= original_updated);
    }
}
