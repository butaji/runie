//! redb-based session persistence.
//!
//! Each session gets its own redb database file: `<dir>/<id>.redb`.
//! Tables:
//!   - `meta`    (key=u32::MAX) → JSON-serialized SessionMetadata
//!   - `events`  (key=u32)       → JSON-serialized DurableCoreEvent
//!
//! Provides atomic batch appends and automatic JSONL migration on open.
//! Wrap `SessionStore` methods in `tokio::task::spawn_blocking` for async contexts.

/// Alias for backward compatibility with SessionActor.
pub use crate::session_index::SessionMetadata as SessionMeta;

use crate::event::durable::DurableCoreEvent;
use crate::session_index::{SessionIndex, SessionMetadata};
use redb::{Database, ReadableTable, TableDefinition};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

#[allow(dead_code)] // reserved for future schema migration checks
const SCHEMA_VERSION: u64 = 1;

// Table definitions
const TABLE_META: TableDefinition<u32, &str> = TableDefinition::new("meta");
const TABLE_EVENTS: TableDefinition<u32, &str> = TableDefinition::new("events");

/// redb-backed session store — each session has its own `.redb` file.
#[derive(Debug, Clone)]
pub struct SessionStore {
    dir: PathBuf,
}

impl SessionStore {
    /// Create a new store at the given directory.
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    /// Path to the redb file for a session.
    pub fn path(&self, session_id: &str) -> PathBuf {
        self.dir.join(format!("{}.redb", session_id))
    }

    /// Open (or create) a session database, migrating from JSONL if needed.
    ///
    /// Returns the database and a flag indicating whether migration occurred.
    fn open_db(path: &Path) -> anyhow::Result<(Database, bool)> {
        let jsonl_path = path.with_extension("jsonl");
        let db_existed = path.exists();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let db = Database::create(path)?;
        let mut migrated = false;

        // Initialize schema
        {
            let tx = db.begin_write()?;
            let _ = tx.open_table(TABLE_META)?;
            let _ = tx.open_table(TABLE_EVENTS)?;
            tx.commit()?;
        }

        // Migrate JSONL if it exists and the DB was just created
        if jsonl_path.exists() && !db_existed {
            let content = fs::read_to_string(&jsonl_path)?;
            let mut seq = 0u32;
            let tx = db.begin_write()?;
            {
                let mut table = tx.open_table(TABLE_EVENTS)?;
                for line in content.lines() {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    if serde_json::from_str::<DurableCoreEvent>(line).is_ok() {
                        table.insert(&seq, line)?;
                        seq += 1;
                    }
                }
            }
            tx.commit()?;
            // Rename JSONL to .jsonl.migrated so it won't be re-imported
            let backup = path.with_extension("jsonl.migrated");
            fs::rename(&jsonl_path, backup).ok();
            migrated = true;
        }

        Ok((db, migrated))
    }

    /// Internal: get max event sequence number in a db.
    fn max_seq(db: &Database) -> anyhow::Result<u32> {
        let tx = db.begin_read()?;
        let table = tx.open_table(TABLE_EVENTS)?;
        let mut max = 0u32;
        for entry in table.iter()? {
            let (k, _) = entry?;
            let seq = k.value();
            if seq > max {
                max = seq;
            }
        }
        Ok(max)
    }

    /// Append a durable event to the session's redb store.
    ///
    /// Wraps in a write transaction. Caller should wrap in `spawn_blocking`
    /// for async contexts.
    pub fn append(&self, session_id: &str, event: &DurableCoreEvent) -> anyhow::Result<()> {
        let path = self.path(session_id);
        let (db, _) = Self::open_db(&path)?;
        let seq = Self::max_seq(&db)? + 1;
        let tx = db.begin_write()?;
        {
            let mut table = tx.open_table(TABLE_EVENTS)?;
            let val = serde_json::to_string(event)?;
            table.insert(&seq, &*val)?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Append multiple events in a single atomic batch.
    pub fn append_batch(
        &self,
        session_id: &str,
        events: &[DurableCoreEvent],
    ) -> anyhow::Result<()> {
        if events.is_empty() {
            return Ok(());
        }
        let path = self.path(session_id);
        let (db, _) = Self::open_db(&path)?;
        let start_seq = Self::max_seq(&db)? + 1;
        let tx = db.begin_write()?;
        {
            let mut table = tx.open_table(TABLE_EVENTS)?;
            for (i, event) in events.iter().enumerate() {
                let val = serde_json::to_string(event)?;
                table.insert(&(start_seq + i as u32), &*val)?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Load all events from a session's redb store in order.
    pub fn load_events(&self, session_id: &str) -> anyhow::Result<Vec<DurableCoreEvent>> {
        let path = self.path(session_id);
        if !path.exists() {
            // Fall back to JSONL for sessions created before migration
            return self.load_events_jsonl(session_id);
        }

        let (db, _) = Self::open_db(&path)?;
        let tx = db.begin_read()?;
        let table = tx.open_table(TABLE_EVENTS)?;

        let mut events = Vec::new();
        let mut keys: Vec<u32> = Vec::new();

        for entry in table.iter()? {
            let (k, v) = entry?;
            keys.push(k.value());
            let val = v.value();
            if let Ok(event) = serde_json::from_str::<DurableCoreEvent>(val) {
                events.push(event);
            }
        }

        // Sort events by their sequence key
        // Rebuild with original keys preserved
        let mut paired: Vec<_> = keys.into_iter().zip(events).collect();
        paired.sort_by_key(|(k, _)| *k);
        events = paired.into_iter().map(|(_, e)| e).collect();

        Ok(events)
    }

    /// Fallback: load events from a JSONL file.
    fn load_events_jsonl(&self, session_id: &str) -> anyhow::Result<Vec<DurableCoreEvent>> {
        let path = self.dir.join(format!("{}.jsonl", session_id));
        if !path.exists() {
            return Ok(Vec::new());
        }
        let file = fs::File::open(&path)?;
        let reader = BufReader::new(file);
        let mut events = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(event) = serde_json::from_str::<DurableCoreEvent>(&line) {
                events.push(event);
            }
        }
        Ok(events)
    }

    /// Delete a session's redb file.
    pub fn delete(&self, session_id: &str) -> anyhow::Result<()> {
        let path = self.path(session_id);
        if path.exists() {
            // Redb requires exclusive access — drop the file handle by dropping
            // any existing db handles first (we open fresh each time)
            fs::remove_file(&path)?;
        }
        // Also clean up migrated JSONL backup
        let backup = path.with_extension("jsonl.migrated");
        fs::remove_file(&backup).ok();
        Ok(())
    }

    /// List all session IDs (from .redb files, falling back to .jsonl).
    pub fn list(&self) -> anyhow::Result<Vec<String>> {
        if !self.dir.exists() {
            return Ok(Vec::new());
        }
        let mut ids = Vec::new();
        for entry in fs::read_dir(&self.dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(id) = name.strip_suffix(".redb") {
                ids.push(id.to_string());
            } else if let Some(id) = name.strip_suffix(".jsonl") {
                ids.push(id.to_string());
            }
        }
        ids.sort();
        Ok(ids)
    }

    /// Update a session's metadata in the redb store.
    pub fn update_index(&self, meta: &SessionMetadata) -> anyhow::Result<()> {
        let path = self.path(&meta.id);
        let (db, _) = Self::open_db(&path)?;
        let tx = db.begin_write()?;
        {
            let mut table = tx.open_table(TABLE_META)?;
            let val = serde_json::to_string(meta)?;
            table.insert(&u32::MAX, &*val)?;
        }
        tx.commit()?;

        // Also sync to the JSON index for backward compat
        let data_dir = self.dir.parent().unwrap_or(&self.dir).to_path_buf();
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

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::DurableCoreEvent;

    fn test_store() -> SessionStore {
        let dir = tempfile::tempdir().unwrap();
        SessionStore::new(dir.path().to_path_buf())
    }

    fn append_msg(store: &SessionStore, sid: &str, mid: &str, role: &str, content: &str, ts: f64) {
        store.append(sid, &DurableCoreEvent::MessageSent {
            id: mid.into(),
            role: role.into(),
            content: content.into(),
            timestamp: ts,
        }).unwrap();
    }

    #[test]
    fn redb_appends_and_replays_events() {
        let store = test_store();
        let sid = "test-replay";

        append_msg(&store, sid, "msg1", "user", "Hello", 1.0);
        append_msg(&store, sid, "msg2", "assistant", "Hi there!", 2.0);
        store.append(sid, &DurableCoreEvent::ModelSwitched {
            provider: "anthropic".into(),
            model: "claude-3".into(),
        }).unwrap();

        let events = store.load_events(sid).unwrap();
        assert_eq!(events.len(), 3);
        assert!(matches!(&events[0], DurableCoreEvent::MessageSent { id, .. } if id == "msg1"));
        assert!(matches!(&events[2], DurableCoreEvent::ModelSwitched { provider, .. } if provider == "anthropic"));
    }

    #[test]
    fn redb_atomic_batch_survives_crash() {
        let store = test_store();
        let sid = "test-crash";

        let batch = vec![
            DurableCoreEvent::MessageSent { id: "1".into(), role: "user".into(), content: "First".into(), timestamp: 1.0 },
            DurableCoreEvent::MessageSent { id: "2".into(), role: "user".into(), content: "Second".into(), timestamp: 2.0 },
            DurableCoreEvent::MessageSent { id: "3".into(), role: "user".into(), content: "Third".into(), timestamp: 3.0 },
        ];

        store.append_batch(sid, &batch).unwrap();

        // Verify all events persisted
        let events = store.load_events(sid).unwrap();
        assert_eq!(events.len(), 3);
        assert!(events.iter().all(|e| matches!(e, DurableCoreEvent::MessageSent { .. })));
    }

    #[test]
    fn redb_migrates_jsonl_session() {
        let dir = tempfile::tempdir().unwrap();
        let dir_path = dir.path().to_path_buf();

        // Create a legacy JSONL file
        let jsonl_path = dir_path.join("legacy-session.jsonl");
        let jsonl_content = concat!(
            r#"{"event":"messageSent","id":"m1","role":"user","content":"Hello","timestamp":1.0}"#,
            "\n",
            r#"{"event":"messageSent","id":"m2","role":"assistant","content":"Hi!","timestamp":2.0}"#,
            "\n"
        );
        std::fs::write(&jsonl_path, jsonl_content).unwrap();

        // Open via SessionStore — should trigger migration
        let store = SessionStore::new(dir_path);
        let events = store.load_events("legacy-session").unwrap();

        assert_eq!(events.len(), 2);
        assert!(matches!(&events[0], DurableCoreEvent::MessageSent { id, .. } if id == "m1"));
        assert!(matches!(&events[1], DurableCoreEvent::MessageSent { id, .. } if id == "m2"));
    }

    #[test]
    fn redb_empty_when_no_file() {
        let store = test_store();
        let events = store.load_events("nonexistent").unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn redb_delete() {
        let store = test_store();
        let sid = "test-delete";

        append_msg(&store, sid, "msg1", "user", "Test", 1.0);
        assert!(store.path(sid).exists());
        store.delete(sid).unwrap();
        assert!(!store.path(sid).exists());
    }

    #[test]
    fn redb_list() {
        let store = test_store();

        append_msg(&store, "session-a", "m1", "user", "A", 1.0);
        append_msg(&store, "session-b", "m2", "user", "B", 2.0);

        let list = store.list().unwrap();
        assert!(list.contains(&"session-a".into()));
        assert!(list.contains(&"session-b".into()));
    }

    #[test]
    fn redb_meta_round_trips() {
        let store = test_store();
        let sid = "test-meta";

        let meta = SessionMetadata {
            id: sid.into(),
            display_name: "My Session".into(),
            created_at: 1000.0,
            updated_at: 2000.0,
            message_count: 5,
            summary: Some("A summary".into()),
            is_starred: true,
            is_system: false,
        };

        store.update_index(&meta).unwrap();

        // Reload via load_events path (meta is stored in redb, load it back)
        let (db, _) = SessionStore::open_db(&store.path(sid)).unwrap();
        let tx = db.begin_read().unwrap();
        let table = tx.open_table(TABLE_META).unwrap();
        let val = table.get(&u32::MAX).unwrap().unwrap();
        let loaded: SessionMetadata = serde_json::from_str(val.value()).unwrap();
        assert_eq!(loaded.display_name, "My Session");
        assert_eq!(loaded.message_count, 5);
        assert!(loaded.is_starred);
    }

    #[test]
    fn redb_multiple_sessions_isolated() {
        let store = test_store();

        store.append("s1", &DurableCoreEvent::MessageSent {
            id: "1".into(), role: "user".into(), content: "S1".into(), timestamp: 1.0,
        }).unwrap();
        store.append("s2", &DurableCoreEvent::MessageSent {
            id: "2".into(), role: "user".into(), content: "S2".into(), timestamp: 2.0,
        }).unwrap();

        let ev1 = store.load_events("s1").unwrap();
        let ev2 = store.load_events("s2").unwrap();

        assert_eq!(ev1.len(), 1);
        assert_eq!(ev2.len(), 1);

        let list = store.list().unwrap();
        assert!(list.contains(&"s1".into()));
        assert!(list.contains(&"s2".into()));
    }
}
