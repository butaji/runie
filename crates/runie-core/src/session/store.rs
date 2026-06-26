//! JSONL-based session persistence.
//!
//! Each session gets its own JSONL file: `<dir>/<id>.jsonl`.
//! Events are appended one per line for simple inspection and debugging.
//!
//! Provides atomic batch appends via temp-file + rename, and automatic
//! migration from redb files on first open (when the `redb-migration` feature is enabled).
//! Wrap `SessionStore` methods in `tokio::task::spawn_blocking` for async contexts.

/// Alias for backward compatibility with SessionActor.
pub use crate::session::index::SessionMetadata as SessionMeta;

use crate::event::durable::DurableCoreEvent;
use crate::session::index::{SessionIndex, SessionMetadata};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

/// JSONL-backed session store — each session has its own `.jsonl` file.
#[derive(Debug, Clone)]
pub struct SessionStore {
    dir: PathBuf,
}

impl SessionStore {
    /// Create a new store at the given directory.
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    /// Default store — uses `RUNIE_SESSIONS_DIR` or OS data dir.
    pub fn default_store() -> Option<Self> {
        if let Ok(dir) = std::env::var("RUNIE_SESSIONS_DIR") {
            return Some(Self::new(PathBuf::from(dir)));
        }
        dirs::data_dir().map(|d| Self::new(d.join("runie").join("sessions")))
    }

    /// Store directory.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Path to the JSONL file for a session.
    pub fn path(&self, session_id: &str) -> PathBuf {
        self.dir.join(format!("{}.jsonl", session_id))
    }

    /// Open (or create) a session, migrating from redb if needed.
    pub(crate) fn open_db(path: &Path) -> anyhow::Result<()> {
        #[cfg(feature = "redb-migration")]
        {
            let redb_path = path.with_extension("redb");
            if redb_path.exists() {
                Self::migrate_redb(&redb_path, path)?;
            }
        }
        Self::ensure_parent_dir(path)
    }

    fn ensure_parent_dir(path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(())
    }

    /// Migrate from a redb file to JSONL.
    #[cfg(feature = "redb-migration")]
    fn migrate_redb(redb_path: &Path, jsonl_path: &Path) -> anyhow::Result<bool> {
        use redb::{Database, ReadableTable, TableDefinition};

        const TABLE_EVENTS: TableDefinition<u32, &str> = TableDefinition::new("events");

        Self::ensure_parent_dir(jsonl_path)?;
        let db = Database::open(redb_path)?;
        let tx = db.begin_read()?;
        let table = tx.open_table(TABLE_EVENTS)?;

        let mut events: Vec<(u32, String)> = Vec::new();
        for entry in table.iter()? {
            let (k, v) = entry?;
            let key = k.value();
            let val = v.value();
            events.push((key, val.to_owned()));
        }
        drop(table);
        drop(tx);

        events.sort_by_key(|(k, _)| *k);

        // Write to temp file then atomically rename
        let temp_path = jsonl_path.with_extension("jsonl.tmp");
        {
            let mut file = File::create(&temp_path)?;
            for (_, val) in &events {
                writeln!(file, "{}", val)?;
            }
            file.sync_all()?;
        }
        fs::rename(&temp_path, jsonl_path)?;

        // Mark redb as migrated
        let backup = redb_path.with_extension("redb.migrated");
        fs::rename(redb_path, backup).ok();

        Ok(true)
    }

    /// Append a durable event to the session's JSONL file.
    ///
    /// Caller should wrap in `spawn_blocking` for async contexts.
    pub fn append(&self, session_id: &str, event: &DurableCoreEvent) -> anyhow::Result<()> {
        let path = self.path(session_id);
        Self::open_db(&path)?;
        let val = serde_json::to_string(event)?;
        let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
        writeln!(file, "{}", val)?;
        file.sync_all()?;
        Ok(())
    }

    /// Append multiple events in a single atomic batch using temp file + rename.
    pub fn append_batch(
        &self,
        session_id: &str,
        events: &[DurableCoreEvent],
    ) -> anyhow::Result<()> {
        if events.is_empty() {
            return Ok(());
        }
        let path = self.path(session_id);
        Self::open_db(&path)?;

        // Write to temp file
        let temp_path = path.with_extension("jsonl.tmp");
        {
            let mut file = File::create(&temp_path)?;
            // First, copy existing content
            if path.exists() {
                let existing = fs::read_to_string(&path)?;
                file.write_all(existing.as_bytes())?;
            }
            // Then append new events
            for event in events {
                let val = serde_json::to_string(event)?;
                writeln!(file, "{}", val)?;
            }
            file.sync_all()?;
        }
        // Atomic rename
        fs::rename(&temp_path, &path)?;
        Ok(())
    }

    /// Load all events from a session's JSONL file in order.
    pub fn load_events(&self, session_id: &str) -> anyhow::Result<Vec<DurableCoreEvent>> {
        let path = self.path(session_id);
        if !path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        let mut events = Vec::new();
        let mut parse_errors = Vec::new();

        for (i, line_result) in reader.lines().enumerate() {
            let line = line_result?;
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            match serde_json::from_str::<DurableCoreEvent>(line) {
                Ok(event) => events.push(event),
                Err(e) => {
                    tracing::warn!("failed to parse session event at line {}: {}", i + 1, e);
                    parse_errors.push(format!("line {}: {}", i + 1, e));
                }
            }
        }

        if !parse_errors.is_empty() {
            return Err(anyhow::anyhow!(
                "session {} contains {} unparseable event(s): {}",
                session_id,
                parse_errors.len(),
                parse_errors.join("; ")
            ));
        }

        Ok(events)
    }

    /// Delete a session's JSONL file.
    pub fn delete(&self, session_id: &str) -> anyhow::Result<()> {
        let path = self.path(session_id);
        if !path.exists() {
            return Err(anyhow::anyhow!("Session '{}' not found", session_id));
        }
        fs::remove_file(&path)?;
        // Also clean up migrated redb backup
        let backup = path.with_extension("redb.migrated");
        fs::remove_file(&backup).ok();
        Ok(())
    }

    /// List all session IDs (from .jsonl files).
    pub fn list(&self) -> anyhow::Result<Vec<String>> {
        if !self.dir.exists() {
            return Ok(Vec::new());
        }
        let mut ids = Vec::new();
        for entry in fs::read_dir(&self.dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(id) = name.strip_suffix(".jsonl") {
                ids.push(id.to_owned());
            }
        }
        ids.sort();
        Ok(ids)
    }

    /// Async version of [`SessionStore::list`] that runs the filesystem scan
    /// on the blocking thread pool.
    pub async fn list_async(&self) -> anyhow::Result<Vec<String>> {
        let store = self.clone();
        tokio::task::spawn_blocking(move || store.list()).await?
    }

    /// Update a session's metadata in the JSON index.
    pub fn update_index(&self, meta: &SessionMetadata) -> anyhow::Result<()> {
        let data_dir = self.dir.clone();
        let mut index = SessionIndex::load(&data_dir).unwrap_or_default();
        index.upsert(meta.clone());
        index.save(&data_dir)
    }

    /// Remove a session from the index.
    pub fn remove_from_index(&self, session_id: &str) -> anyhow::Result<()> {
        self.delete(session_id)?;
        let data_dir = self.dir.parent().unwrap_or(&self.dir).to_path_buf();
        let mut index = SessionIndex::load(&data_dir).unwrap_or_default();
        index.remove(session_id);
        index.save(&data_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::DurableCoreEvent;

    fn test_store() -> SessionStore {
        let dir = tempfile::tempdir().unwrap();
        SessionStore::new(dir.path().to_path_buf())
    }

    fn append_msg(store: &SessionStore, sid: &str, mid: &str, role: &str, content: &str, ts: f64) {
        store
            .append(
                sid,
                &DurableCoreEvent::MessageSent {
                    id: mid.into(),
                    role: role.into(),
                    content: content.into(),
                    timestamp: ts,
                    provider: String::new(),
                },
            )
            .unwrap();
    }

    #[test]
    fn append_event_writes_jsonl_line() {
        let store = test_store();
        let sid = "test-append";
        append_msg(&store, sid, "msg1", "user", "Hello", 1.0);

        let path = store.path(sid);
        let content = fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 1, "should have exactly one line");
        assert!(content.contains("Hello"), "should contain event content");
    }

    #[test]
    fn load_events_round_trips() {
        let store = test_store();
        let sid = "test-roundtrip";

        append_msg(&store, sid, "msg1", "user", "Hello", 1.0);
        append_msg(&store, sid, "msg2", "assistant", "Hi!", 2.0);

        let events = store.load_events(sid).unwrap();
        assert_eq!(events.len(), 2);
        assert!(matches!(
            &events[0],
            DurableCoreEvent::MessageSent { id, .. } if id == "msg1"
        ));
        assert!(matches!(
            &events[1],
            DurableCoreEvent::MessageSent { id, .. } if id == "msg2"
        ));
    }

    #[test]
    fn append_batch_atomic() {
        let store = test_store();
        let sid = "test-batch";

        let batch = vec![
            DurableCoreEvent::MessageSent {
                id: "1".into(),
                role: "user".into(),
                content: "First".into(),
                timestamp: 1.0,
                provider: String::new(),
            },
            DurableCoreEvent::MessageSent {
                id: "2".into(),
                role: "user".into(),
                content: "Second".into(),
                timestamp: 2.0,
                provider: String::new(),
            },
            DurableCoreEvent::MessageSent {
                id: "3".into(),
                role: "user".into(),
                content: "Third".into(),
                timestamp: 3.0,
                provider: String::new(),
            },
        ];

        store.append_batch(sid, &batch).unwrap();

        let events = store.load_events(sid).unwrap();
        assert_eq!(events.len(), 3);
        assert!(events
            .iter()
            .all(|e| matches!(e, DurableCoreEvent::MessageSent { .. })));
    }

    #[test]
    fn index_load_handles_missing_file() {
        let store = test_store();
        let events = store.load_events("nonexistent").unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn delete_session() {
        let store = test_store();
        let sid = "test-delete";

        append_msg(&store, sid, "msg1", "user", "Test", 1.0);
        assert!(store.path(sid).exists());
        store.delete(sid).unwrap();
        assert!(!store.path(sid).exists());
    }

    #[test]
    fn list_sessions() {
        let store = test_store();

        append_msg(&store, "session-a", "m1", "user", "A", 1.0);
        append_msg(&store, "session-b", "m2", "user", "B", 2.0);

        let list = store.list().unwrap();
        assert!(list.contains(&"session-a".into()));
        assert!(list.contains(&"session-b".into()));
    }

    #[test]
    fn multiple_sessions_isolated() {
        let store = test_store();

        store
            .append(
                "s1",
                &DurableCoreEvent::MessageSent {
                    id: "1".into(),
                    role: "user".into(),
                    content: "S1".into(),
                    timestamp: 1.0,
                    provider: String::new(),
                },
            )
            .unwrap();
        store
            .append(
                "s2",
                &DurableCoreEvent::MessageSent {
                    id: "2".into(),
                    role: "user".into(),
                    content: "S2".into(),
                    timestamp: 2.0,
                    provider: String::new(),
                },
            )
            .unwrap();

        let ev1 = store.load_events("s1").unwrap();
        let ev2 = store.load_events("s2").unwrap();

        assert_eq!(ev1.len(), 1);
        assert_eq!(ev2.len(), 1);

        let list = store.list().unwrap();
        assert!(list.contains(&"s1".into()));
        assert!(list.contains(&"s2".into()));
    }

    #[test]
    fn load_events_returns_ordered_events() {
        let store = test_store();
        let sid = "test-order";
        append_msg(&store, sid, "a", "user", "first", 1.0);
        append_msg(&store, sid, "b", "user", "second", 2.0);
        let events = store.load_events(sid).unwrap();
        assert_eq!(events.len(), 2);
        assert!(matches!(
            &events[0],
            DurableCoreEvent::MessageSent { id, .. } if id == "a"
        ));
        assert!(matches!(
            &events[1],
            DurableCoreEvent::MessageSent { id, .. } if id == "b"
        ));
    }

    #[test]
    fn load_events_rejects_malformed_json() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::new(dir.path().to_path_buf());
        let path = store.path("corrupt");
        fs::write(&path, "valid event\nnot json at all\n").unwrap();

        let result = store.load_events("corrupt");
        assert!(result.is_err(), "parse failure should be an error");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("unparseable"),
            "error message should mention unparseable, got: {}",
            err
        );
    }
}
