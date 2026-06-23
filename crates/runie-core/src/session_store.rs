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
pub(crate) const TABLE_META: TableDefinition<u32, &str> = TableDefinition::new("meta");
pub(crate) const TABLE_EVENTS: TableDefinition<u32, &str> = TableDefinition::new("events");

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

    /// Path to the redb file for a session.
    pub fn path(&self, session_id: &str) -> PathBuf {
        self.dir.join(format!("{}.redb", session_id))
    }

    /// Open (or create) a session database, migrating from JSONL if needed.
    pub(crate) fn open_db(path: &Path) -> anyhow::Result<(Database, bool)> {
        let jsonl_path = path.with_extension("jsonl");
        let db_existed = path.exists();
        Self::ensure_parent_dir(path)?;
        let db = Database::create(path)?;
        Self::init_db_tables(&db)?;
        let migrated = if jsonl_path.exists() && !db_existed {
            Self::migrate_jsonl(&db, &jsonl_path, path)?
        } else {
            false
        };
        Ok((db, migrated))
    }

    fn ensure_parent_dir(path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(())
    }
    fn init_db_tables(db: &Database) -> anyhow::Result<()> {
        let tx = db.begin_write()?;
        let _ = tx.open_table(TABLE_META)?;
        let _ = tx.open_table(TABLE_EVENTS)?;
        tx.commit()?;
        Ok(())
    }
    fn migrate_jsonl(db: &Database, jsonl_path: &Path, db_path: &Path) -> anyhow::Result<bool> {
        let content = fs::read_to_string(jsonl_path)?;
        let tx = db.begin_write()?;
        {
            let mut table = tx.open_table(TABLE_EVENTS)?;
            let mut seq = 0u32;
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
        let backup = db_path.with_extension("jsonl.migrated");
        fs::rename(jsonl_path, backup).ok();
        Ok(true)
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
            return self.load_events_jsonl(session_id);
        }

        let (db, _) = Self::open_db(&path)?;
        let tx = db.begin_read()?;
        let table = tx.open_table(TABLE_EVENTS)?;

        let mut paired = Vec::new();
        let mut parse_errors = Vec::new();
        for entry in table.iter()? {
            let (k, v) = entry?;
            let key = k.value();
            let val = v.value();
            match serde_json::from_str::<DurableCoreEvent>(val) {
                Ok(event) => paired.push((key, event)),
                Err(e) => {
                    tracing::warn!("failed to parse session event at key {}: {}", key, e);
                    parse_errors.push(format!("key {}: {}", key, e));
                }
            }
        }
        drop(table);
        drop(tx);

        if !parse_errors.is_empty() {
            return Err(anyhow::anyhow!(
                "session {} contains {} unparseable event(s): {}",
                session_id,
                parse_errors.len(),
                parse_errors.join("; ")
            ));
        }

        paired.sort_by_key(|(k, _)| *k);
        let events: Vec<_> = paired.into_iter().map(|(_, e)| e).collect();
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
        let mut parse_errors = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<DurableCoreEvent>(&line) {
                Ok(event) => events.push(event),
                Err(e) => parse_errors.push(format!("{}", e)),
            }
        }
        if !parse_errors.is_empty() {
            return Err(anyhow::anyhow!(
                "session {} JSONL contains {} unparseable line(s): {}",
                session_id,
                parse_errors.len(),
                parse_errors.join("; ")
            ));
        }
        Ok(events)
    }

    /// Delete a session's redb file.
    pub fn delete(&self, session_id: &str) -> anyhow::Result<()> {
        let path = self.path(session_id);
        if !path.exists() {
            return Err(anyhow::anyhow!("Session '{}' not found", session_id));
        }
        // Redb requires exclusive access — drop the file handle by dropping
        // any existing db handles first (we open fresh each time)
        fs::remove_file(&path)?;
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

    /// Async version of [`SessionStore::list`] that runs the filesystem scan
    /// on the blocking thread pool.
    pub async fn list_async(&self) -> anyhow::Result<Vec<String>> {
        let store = self.clone();
        tokio::task::spawn_blocking(move || store.list()).await?
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
