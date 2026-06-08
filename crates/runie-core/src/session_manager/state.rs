//! Session Manager State

use crate::event_bus::DomainEvent;
use crate::session_jsonl::{JsonlWriter, SessionMeta};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Session manager state — mutable only within the actor loop
#[derive(Debug)]
#[derive(Default)]
pub struct SessionState {
    /// Current session name (if any)
    pub session_name: Option<String>,
    /// Current JSONL writer (None if no active session)
    writer: Option<JsonlWriter>,
    /// Pending events to replay on load
    pending_events: VecDeque<DomainEvent>,
    /// Last snapshot timestamp
    last_snapshot: Option<Instant>,
    /// Whether session is dirty
    is_dirty: bool,
}


impl SessionState {
    /// Start a new session
    pub fn start_session(
        &mut self,
        name: String,
        provider: String,
        model: String,
    ) -> anyhow::Result<()> {
        self.close_session()?;

        let meta = SessionMeta::new(name.clone(), provider, model);
        let path = crate::session_jsonl::session_path(&name)
            .ok_or_else(|| anyhow::anyhow!("No sessions directory"))?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        self.writer = Some(JsonlWriter::create(&path, &meta)?);
        self.session_name = Some(name);
        self.pending_events.clear();
        self.last_snapshot = Some(Instant::now());
        self.is_dirty = true;
        Ok(())
    }

    /// Resume an existing session (append mode)
    pub fn resume_session(&mut self, name: String) -> anyhow::Result<SessionMeta> {
        self.close_session()?;

        let path = crate::session_jsonl::session_path(&name)
            .ok_or_else(|| anyhow::anyhow!("No sessions directory"))?;

        self.writer = Some(JsonlWriter::append(&path)?);
        self.session_name = Some(name.clone());
        self.is_dirty = true;

        let mut reader = crate::session_jsonl::JsonlReader::open(&path)?;
        let meta = reader.read_meta()?;
        Ok(meta)
    }

    /// Close current session
    pub fn close_session(&mut self) -> anyhow::Result<()> {
        if let Some(mut writer) = self.writer.take() {
            writer.flush()?;
        }
        self.session_name = None;
        self.is_dirty = false;
        Ok(())
    }

    /// Record a domain event (append to JSONL)
    pub fn record_event(&mut self, event: &DomainEvent) -> anyhow::Result<()> {
        if let Some(ref mut writer) = self.writer {
            writer.write_event(event)?;
            self.is_dirty = true;
        }
        self.pending_events.push_back(event.clone());
        Ok(())
    }

    /// Check if snapshot is needed
    pub fn needs_snapshot(&self, interval: Duration) -> bool {
        self.last_snapshot.map(|t| t.elapsed() >= interval).unwrap_or(true)
    }

    /// Mark snapshot taken
    pub fn mark_snapshot(&mut self) {
        self.last_snapshot = Some(Instant::now());
    }

    /// Get pending events for replay
    pub fn take_pending_events(&mut self) -> Vec<DomainEvent> {
        self.pending_events.drain(..).collect()
    }

    /// Check if session is active
    pub fn has_active_session(&self) -> bool {
        self.session_name.is_some()
    }

    /// Check if session is dirty
    pub fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    /// Flush pending writes
    pub fn flush(&mut self) -> anyhow::Result<()> {
        if let Some(ref mut writer) = self.writer {
            writer.flush()?;
            self.is_dirty = false;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state() -> SessionState {
        SessionState::default()
    }

    #[test]
    fn test_default_state() {
        let mut state = make_state();
        assert!(!state.has_active_session());
        assert!(!state.is_dirty());
        assert!(state.take_pending_events().is_empty());
    }
}
