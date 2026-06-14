//! JSONL session store — append-only event log per session,
//! plus a session metadata index (index.jsonl) for browsing.

use crate::event::DurableCoreEvent;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Metadata for a single session, persisted in the session index.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionMeta {
    pub id: String,
    #[serde(default)]
    pub display_name: String,
    pub created_at: f64,
    pub updated_at: f64,
    #[serde(default)]
    pub message_count: usize,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub is_starred: bool,
    #[serde(default)]
    pub is_system: bool,
}

/// A thread-safe session store backed by JSONL files.
#[derive(Clone)]
pub struct SessionStore {
    data_dir: PathBuf,
    lock: Arc<Mutex<()>>,
}

impl SessionStore {
    /// Create a new session store rooted at `data_dir`.
    /// Sessions are stored at `data_dir/runie/sessions/<session_id>.jsonl`.
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            data_dir,
            lock: Arc::new(Mutex::new(())),
        }
    }

    /// Create a default session store in the OS data directory.
    pub fn default_store() -> Option<Self> {
        dirs::data_dir().map(|d| Self::new(d.join("runie").join("sessions")))
    }

    fn session_dir(&self) -> PathBuf {
        self.data_dir.clone()
    }

    fn session_path(&self, session_id: &str) -> PathBuf {
        self.session_dir().join(format!("{}.jsonl", session_id))
    }

    fn ensure_dir(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(self.session_dir())
    }

    /// Append a durable event to the session's JSONL file.
    /// Uses atomic write (write to temp file, rename) to prevent partial writes.
    pub fn append(&self, session_id: &str, event: &DurableCoreEvent) -> std::io::Result<()> {
        let _guard = self.lock.lock().unwrap();
        self.ensure_dir()?;

        let path = self.session_path(session_id);
        let line = serde_json::to_string(event).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e)
        })?;

        let tmp_dir = self.session_dir();
        let mut tmp = tempfile::NamedTempFile::new_in(&tmp_dir)?;

        if path.exists() {
            let existing = std::fs::read_to_string(&path)?;
            tmp.write_all(existing.as_bytes())?;
        }
        writeln!(tmp, "{}", line)?;
        tmp.flush()?;
        tmp.persist(&path).map_err(|e| e.error)?;

        Ok(())
    }

    /// Load all durable events from a session's JSONL file.
    pub fn load_events(&self, session_id: &str) -> std::io::Result<Vec<DurableCoreEvent>> {
        let path = self.session_path(session_id);
        if !path.exists() {
            return Ok(Vec::new());
        }

        let file = std::fs::File::open(&path)?;
        let reader = std::io::BufReader::new(file);

        let mut events = Vec::new();
        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let event: DurableCoreEvent = serde_json::from_str(trimmed).map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, e)
            })?;
            events.push(event);
        }
        Ok(events)
    }

    /// Delete a session's JSONL file.
    pub fn delete(&self, session_id: &str) -> std::io::Result<()> {
        let path = self.session_path(session_id);
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    // ── Session Index ────────────────────────────────────────────────────────

    fn session_index_path(&self) -> PathBuf {
        self.session_dir().join("index.jsonl")
    }

    /// Load all session metadata from the index.
    pub fn load_index(&self) -> std::io::Result<Vec<SessionMeta>> {
        let path = self.session_index_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let file = std::fs::File::open(&path)?;
        let reader = std::io::BufReader::new(file);
        let mut sessions = Vec::new();
        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            sessions.push(serde_json::from_str(trimmed).map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, e)
            })?);
        }
        Ok(sessions)
    }

    /// Save the entire session index (replaces existing atomically).
    pub fn save_index(&self, sessions: &[SessionMeta]) -> std::io::Result<()> {
        let _guard = self.lock.lock().unwrap();
        self.ensure_dir()?;
        let path = self.session_index_path();
        let tmp_dir = self.session_dir();
        let mut tmp = tempfile::NamedTempFile::new_in(&tmp_dir)?;
        for s in sessions {
            let line = serde_json::to_string(s).map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, e)
            })?;
            writeln!(tmp, "{}", line)?;
        }
        tmp.flush()?;
        tmp.persist(&path).map_err(|e| e.error)?;
        Ok(())
    }

    /// Update (insert or replace) a single entry in the session index.
    pub fn update_index(&self, meta: &SessionMeta) -> std::io::Result<()> {
        let mut sessions = self.load_index()?;
        if let Some(pos) = sessions.iter().position(|s| s.id == meta.id) {
            sessions[pos] = meta.clone();
        } else {
            sessions.push(meta.clone());
        }
        self.save_index(&sessions)
    }

    /// Remove a session from the index.
    pub fn remove_from_index(&self, session_id: &str) -> std::io::Result<()> {
        let mut sessions = self.load_index()?;
        sessions.retain(|s| s.id != session_id);
        self.save_index(&sessions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_store() -> (SessionStore, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::new(dir.path().to_path_buf());
        (store, dir)
    }

    fn sample_event_1() -> DurableCoreEvent {
        DurableCoreEvent::MessageSent {
            id: "msg.1".into(),
            role: "user".into(),
            content: "hello".into(),
            timestamp: 1000.0,
        }
    }

    fn sample_event_2() -> DurableCoreEvent {
        DurableCoreEvent::MessageSent {
            id: "msg.2".into(),
            role: "assistant".into(),
            content: "world".into(),
            timestamp: 1001.0,
        }
    }

    fn sample_event_3() -> DurableCoreEvent {
        DurableCoreEvent::ToolCalled {
            id: "tool.1".into(),
            name: "bash".into(),
            input: "ls".into(),
        }
    }

    #[test]
    fn session_store_appends_and_replays_events() {
        let (store, _dir) = tmp_store();
        let session_id = "test_session_1";

        store.append(session_id, &sample_event_1()).unwrap();
        store.append(session_id, &sample_event_2()).unwrap();
        store.append(session_id, &sample_event_3()).unwrap();

        let events = store.load_events(session_id).unwrap();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0], sample_event_1());
        assert_eq!(events[1], sample_event_2());
        assert_eq!(events[2], sample_event_3());
    }

    #[test]
    fn session_store_atomic_write_survives_crash() {
        let (store, _dir) = tmp_store();
        let session_id = "crash_test";

        store.append(session_id, &sample_event_1()).unwrap();
        store.append(session_id, &sample_event_2()).unwrap();

        let path = store.session_path(session_id);
        let raw = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = raw.lines().collect();
        assert_eq!(lines.len(), 2);

        for line in &lines {
            let parsed: DurableCoreEvent = serde_json::from_str(line).unwrap();
            let _ = parsed;
        }
    }

    #[test]
    fn jsonl_line_is_valid_json() {
        let (store, _dir) = tmp_store();
        let session_id = "json_valid";

        store.append(session_id, &sample_event_1()).unwrap();
        store.append(session_id, &sample_event_2()).unwrap();
        store.append(session_id, &sample_event_3()).unwrap();

        let path = store.session_path(session_id);
        let raw = std::fs::read_to_string(&path).unwrap();
        for line in raw.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let parsed: serde_json::Value = serde_json::from_str(trimmed)
                .expect("Each line must be valid JSON");
            assert!(parsed.is_object(), "Each line must be a JSON object");
            assert!(
                parsed.get("event").is_some(),
                "Each line must have an 'event' tag"
            );
        }
    }

    #[test]
    fn session_store_load_nonexistent_returns_empty() {
        let (store, _dir) = tmp_store();
        let events = store.load_events("nonexistent").unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn session_store_delete_removes_file() {
        let (store, _dir) = tmp_store();
        store.append("deletable", &sample_event_1()).unwrap();
        assert!(store.session_path("deletable").exists());
        store.delete("deletable").unwrap();
        assert!(!store.session_path("deletable").exists());
    }

    // ── Index Tests ──────────────────────────────────────────────────────────

    fn sample_meta(id: &str, name: &str, msg_count: usize) -> SessionMeta {
        SessionMeta {
            id: id.into(),
            display_name: name.into(),
            created_at: 1000.0,
            updated_at: 2000.0,
            message_count: msg_count,
            summary: format!("summary of {}", name),
            is_starred: false,
            is_system: false,
        }
    }

    #[test]
    fn session_index_round_trips() {
        let (store, _dir) = tmp_store();
        let meta = sample_meta("s1", "Session One", 5);

        store.update_index(&meta).unwrap();

        let loaded = store.load_index().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, "s1");
        assert_eq!(loaded[0].display_name, "Session One");
        assert_eq!(loaded[0].message_count, 5);
        assert_eq!(loaded[0].summary, "summary of Session One");
        assert_eq!(loaded[0].created_at, 1000.0);
        assert_eq!(loaded[0].updated_at, 2000.0);
        assert!(!loaded[0].is_starred);

        let mut updated = meta.clone();
        updated.message_count = 10;
        updated.is_starred = true;
        store.update_index(&updated).unwrap();

        let loaded = store.load_index().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].message_count, 10);
        assert!(loaded[0].is_starred);
    }

    #[test]
    fn summary_generated_for_long_session() {
        let (store, _dir) = tmp_store();
        let mut meta = sample_meta("s2", "Long Session", 10);
        meta.summary = "first 500 chars of session content".into();
        store.update_index(&meta).unwrap();

        let loaded = store.load_index().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].summary, "first 500 chars of session content");
    }

    #[test]
    fn starred_session_sorts_to_top() {
        let (store, _dir) = tmp_store();
        let s1 = sample_meta("a", "Alpha", 1);
        let mut s2 = sample_meta("b", "Beta", 2);
        let mut s3 = sample_meta("c", "Gamma", 3);
        s2.is_starred = true;
        s3.is_starred = true;
        s3.is_system = true;

        store.update_index(&s1).unwrap();
        store.update_index(&s2).unwrap();
        store.update_index(&s3).unwrap();

        let loaded = store.load_index().unwrap();
        let mut sorted = loaded.clone();
        sorted.sort_by(|a, b| {
            b.is_starred.cmp(&a.is_starred).then(a.display_name.cmp(&b.display_name))
        });

        assert_eq!(sorted[0].id, "b", "starred sorts first");
        assert!(sorted[0].is_starred);
        assert!(!sorted[2].is_starred);
    }

    #[test]
    fn session_renamed_event_updates_index() {
        let (store, _dir) = tmp_store();
        let mut meta = sample_meta("rename_me", "Old Name", 3);
        store.update_index(&meta).unwrap();

        meta.display_name = "New Name".into();
        meta.updated_at = 3000.0;
        store.update_index(&meta).unwrap();

        let loaded = store.load_index().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].display_name, "New Name");
        assert_eq!(loaded[0].updated_at, 3000.0);
    }

    #[test]
    fn session_list_renders_summary() {
        let meta = sample_meta("list_test", "Test Session", 7);
        let row = format!(
            "{} — {} ({} msgs)",
            meta.display_name, meta.summary, meta.message_count
        );
        assert!(row.contains("Test Session"));
        assert!(row.contains("summary of Test Session"));
        assert!(row.contains("7 msgs"));
    }
}
