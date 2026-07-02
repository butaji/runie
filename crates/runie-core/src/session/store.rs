//! Unified session persistence with `fs2` advisory locks.
//!
//! Each session is stored in a single file: `<dir>/<id>.jsonl`
//! The file format is:
//!   - Line 1: JSON header (session metadata)
//!   - Remaining lines: JSONL events (one per line)
//!
//! Uses `fs2` advisory locks for cross-process synchronization.

use crate::event::durable::DurableCoreEvent;
use crate::session::persistence::{
    exclusive_lock, read_header, shared_lock, touch_header, write_header, ExclusiveLock,
    SessionHeader, SharedLock,
};
use crate::session::SessionMetadata;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

/// Alias for backward compatibility.
pub use crate::session::SessionMetadata as SessionMeta;

// ── Compaction thresholds ──────────────────────────────────────────────────────

/// Default max file size before compaction is triggered (10 MB).
/// Sessions larger than this are compacted on the next write.
pub const COMPACT_FILE_SIZE_BYTES: u64 = 10 * 1024 * 1024;

/// Default max event count before compaction is triggered (500 events).
/// Prevents unbounded growth even for small messages.
pub const COMPACT_EVENT_COUNT: usize = 500;

/// Default max turn count before compaction is triggered (50 turns).
/// Sessions with many turns accumulate context overhead.
pub const COMPACT_TURN_COUNT: usize = 50;

/// Target event count after compaction (100 events).
/// Compaction summarises the oldest events and keeps the most recent window.
pub const COMPACT_TARGET_EVENTS: usize = 100;

/// Compaction policy: how to preserve the original journal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompactionPolicy {
    /// Default to archive so the original journal is never lost.
    #[default]
    ArchiveToSidecar,
    /// Append a synthetic `SessionCompacted` event to the compacted file.
    /// The original journal is NOT preserved (lossy compaction).
    DiscardOriginal,
}

/// JSONL-backed session store with fs2 advisory locks.
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
    pub fn dir(&self) -> &PathBuf {
        &self.dir
    }

    /// Path to the session file for a given ID.
    pub fn path(&self, session_id: &str) -> PathBuf {
        self.dir.join(format!("{}.jsonl", session_id))
    }

    /// Ensure the parent directory exists.
    fn ensure_parent_dir(&self) -> anyhow::Result<()> {
        fs::create_dir_all(&self.dir)?;
        Ok(())
    }

    /// Open (or create) a session file.
    pub(crate) fn open_db(path: &PathBuf) -> anyhow::Result<()> {
        if path.exists() {
            return Ok(());
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        File::create(path)?;
        Ok(())
    }

    /// Append a durable event to the session's file.
    ///
    /// Caller should wrap in `spawn_blocking` for async contexts.
    #[allow(clippy::let_underscore_lock)]
    pub fn append(&self, session_id: &str, event: &DurableCoreEvent) -> anyhow::Result<()> {
        let path = self.path(session_id);
        self.ensure_parent_dir()?;
        Self::open_db(&path)?;

        let _lock: ExclusiveLock = exclusive_lock(&path)?;
        let mut file = OpenOptions::new().append(true).open(&path)?;
        let val = serde_json::to_string(event)?;
        writeln!(file, "{}", val)?;
        file.sync_all()?;

        touch_header(&path)?;
        Ok(())
    }

    /// Append multiple events in a single atomic batch.
    #[allow(clippy::let_underscore_lock)]
    pub fn append_batch(
        &self,
        session_id: &str,
        events: &[DurableCoreEvent],
    ) -> anyhow::Result<()> {
        if events.is_empty() {
            return Ok(());
        }
        let path = self.path(session_id);
        self.ensure_parent_dir()?;
        Self::open_db(&path)?;

        let _lock: ExclusiveLock = exclusive_lock(&path)?;
        let header = read_header(&path)?;

        let mut file = OpenOptions::new().append(true).open(&path)?;
        for event in events {
            let val = serde_json::to_string(event)?;
            writeln!(file, "{}", val)?;
        }
        file.sync_all()?;

        if header.is_some() {
            touch_header(&path)?;
        }
        Ok(())
    }

    /// Load all events from a session's file.
    #[allow(clippy::let_underscore_lock)]
    pub fn load_events(&self, session_id: &str) -> anyhow::Result<Vec<DurableCoreEvent>> {
        let path = self.path(session_id);
        if !path.exists() {
            return Ok(Vec::new());
        }

        let _lock: SharedLock = shared_lock(&path)?;
        load_events_internal(&path)
    }

    /// Load session metadata from a session's file header.
    #[allow(clippy::let_underscore_lock)]
    pub fn load_metadata(&self, session_id: &str) -> anyhow::Result<Option<SessionMetadata>> {
        let path = self.path(session_id);
        if !path.exists() {
            return Ok(None);
        }

        let _lock: SharedLock = shared_lock(&path)?;
        read_header(&path)
    }

    /// Delete a session's file.
    pub fn delete(&self, session_id: &str) -> anyhow::Result<()> {
        let path = self.path(session_id);
        if !path.exists() {
            return Err(anyhow::anyhow!("Session '{}' not found", session_id));
        }
        fs::remove_file(&path)?;
        Ok(())
    }

    /// Check if a session exists.
    pub fn exists(&self, session_id: &str) -> bool {
        self.path(session_id).exists()
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

    /// Async version of [`SessionStore::list`].
    pub async fn list_async(&self) -> anyhow::Result<Vec<String>> {
        let store = self.clone();
        tokio::task::spawn_blocking(move || store.list()).await?
    }

    /// Load all session metadata from the store.
    pub fn list_metadata(&self) -> anyhow::Result<Vec<SessionMetadata>> {
        let ids = self.list()?;
        let mut results = Vec::with_capacity(ids.len());

        for id in &ids {
            if let Ok(Some(meta)) = self.load_metadata(id) {
                results.push(meta);
            }
        }

        results.sort_by(|a, b| {
            b.updated_at
                .partial_cmp(&a.updated_at)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(results)
    }

    /// Update a session's metadata.
    #[allow(clippy::let_underscore_lock)]
    pub fn update_metadata(&self, meta: &SessionMetadata) -> anyhow::Result<()> {
        let path = self.path(&meta.id);

        if !path.exists() {
            self.ensure_parent_dir()?;
            let header = meta;
            let mut file = File::create(&path)?;
            writeln!(file, "{}", serde_json::to_string(&header)?)?;
            file.sync_all()?;
            return Ok(());
        }

        let _lock: ExclusiveLock = exclusive_lock(&path)?;
        // Write the full metadata (not just touching updated_at)
        write_header(&path, meta)?;
        Ok(())
    }

    /// Remove a session from the store.
    pub fn remove(&self, session_id: &str) -> anyhow::Result<()> {
        self.delete(session_id)?;
        Ok(())
    }
}

// ============================================================================
// Internal helpers
// ============================================================================

fn parse_event_line(line: &str, _line_num: usize) -> Option<DurableCoreEvent> {
    serde_json::from_str::<DurableCoreEvent>(line).ok()
}

fn is_header_line(line: &str) -> bool {
    serde_json::from_str::<SessionHeader>(line).is_ok()
}

fn read_next_line(reader: &mut BufReader<File>, line_num: &mut usize) -> Option<String> {
    let mut line = String::new();
    match reader.read_line(&mut line) {
        Ok(0) => None,
        Ok(_) => {
            *line_num += 1;
            Some(line.trim().to_string())
        }
        Err(e) => {
            tracing::warn!("failed to read line {}: {}", *line_num + 1, e);
            None
        }
    }
}

fn load_events_internal(path: &PathBuf) -> anyhow::Result<Vec<DurableCoreEvent>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut events = Vec::new();
    let mut parse_errors = Vec::new();
    let mut line_num = 0usize;

    while let Some(line) = read_next_line(&mut reader, &mut line_num) {
        if line_num == 1 && is_header_line(&line) {
            continue;
        }
        if line.is_empty() {
            continue;
        }
        match parse_event_line(&line, line_num) {
            Some(event) => events.push(event),
            None => {
                tracing::warn!("failed to parse event at line {}", line_num);
                parse_errors.push(format!("line {}", line_num));
            }
        }
    }

    if !parse_errors.is_empty() && events.is_empty() {
        return Err(anyhow::anyhow!(
            "session contains {} unparseable event(s): {}",
            parse_errors.len(),
            parse_errors.join("; ")
        ));
    }

    Ok(events)
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

    fn make_event(id: &str, role: &str, content: &str, ts: f64) -> DurableCoreEvent {
        DurableCoreEvent::MessageSent {
            id: id.into(),
            role: role.into(),
            content: content.into(),
            timestamp: ts,
            provider: String::new(),
        }
    }

    #[test]
    fn append_event_writes_jsonl_line() {
        let store = test_store();
        let sid = "test-append";
        append_msg(&store, sid, "msg1", "user", "Hello", 1.0);

        let path = store.path(sid);
        let content = fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        // First event may or may not have a header depending on whether
        // the session was previously initialized with metadata
        assert!(!lines.is_empty(), "should have at least one line");
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
            make_event("1", "user", "First", 1.0),
            make_event("2", "user", "Second", 2.0),
            make_event("3", "user", "Third", 3.0),
        ];
        store.append_batch(sid, &batch).unwrap();

        let events = store.load_events(sid).unwrap();
        assert_eq!(events.len(), 3);
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
            .append("s1", &make_event("1", "user", "S1", 1.0))
            .unwrap();
        store
            .append("s2", &make_event("2", "user", "S2", 2.0))
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
        assert!(matches!(&events[0], DurableCoreEvent::MessageSent { id, .. } if id == "a"));
        assert!(matches!(&events[1], DurableCoreEvent::MessageSent { id, .. } if id == "b"));
    }

    #[test]
    fn update_metadata_creates_header() {
        let store = test_store();
        let meta = SessionMetadata {
            id: "test-meta".into(),
            display_name: "Test Session".into(),
            created_at: 1000.0,
            updated_at: 2000.0,
            message_count: 5,
            summary: Some("Test summary".into()),
            is_starred: true,
            is_system: false,
        };
        store.update_metadata(&meta).unwrap();

        let loaded = store.load_metadata("test-meta").unwrap().unwrap();
        assert_eq!(loaded.display_name, "Test Session");
        assert!(loaded.is_starred);
        assert_eq!(loaded.summary.as_deref(), Some("Test summary"));
    }

    #[test]
    fn list_metadata_returns_all_sessions() {
        let store = test_store();
        store
            .update_metadata(&SessionMetadata {
                id: "meta1".into(),
                display_name: "First".into(),
                created_at: 1000.0,
                updated_at: 1000.0,
                message_count: 1,
                summary: None,
                is_starred: false,
                is_system: false,
            })
            .unwrap();
        store
            .update_metadata(&SessionMetadata {
                id: "meta2".into(),
                display_name: "Second".into(),
                created_at: 2000.0,
                updated_at: 2000.0,
                message_count: 2,
                summary: None,
                is_starred: false,
                is_system: false,
            })
            .unwrap();

        let metas = store.list_metadata().unwrap();
        assert_eq!(metas.len(), 2);
        assert_eq!(metas[0].id, "meta2");
        assert_eq!(metas[1].id, "meta1");
    }

    #[test]
    fn compaction_thresholds_are_reasonable() {
        // File size: 10 MB threshold
        assert_eq!(COMPACT_FILE_SIZE_BYTES, 10 * 1024 * 1024);
        // Event count: 500 events before compaction
        assert_eq!(COMPACT_EVENT_COUNT, 500);
        // Turn count: 50 turns before compaction
        assert_eq!(COMPACT_TURN_COUNT, 50);
        // Target: keep last 100 events after compaction
        assert_eq!(COMPACT_TARGET_EVENTS, 100);
        // Thresholds should be ordered correctly
        assert!(COMPACT_TARGET_EVENTS < COMPACT_EVENT_COUNT);
    }

    #[test]
    fn compaction_policy_default_is_archive() {
        let policy = CompactionPolicy::default();
        assert_eq!(policy, CompactionPolicy::ArchiveToSidecar);
    }
}
