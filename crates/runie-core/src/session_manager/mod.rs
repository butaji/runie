//! Session Manager — persists domain events to JSONL, handles session lifecycle.
//!
//! Architecture:
//!   - Subscribes to domain events from the event bus
//!   - Appends events to JSONL file on each domain event
//!   - Supports session load/replay/list/delete
//!   - Periodic snapshots as load accelerators

mod commands;
pub use commands::{SessionCmd, SessionResponse};

mod state;
pub use state::SessionState;

pub use crate::session_jsonl::{SessionMeta, list_session_names, load_session as load, delete_session as delete};

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::event_bus::{ActorChannel, BusEventEnvelope, DomainEvent, EventBus};
use crate::session_jsonl as jsonl;

// ---------------------------------------------------------------------------
// Session Manager Actor
// ---------------------------------------------------------------------------

/// Run the session manager actor loop.
/// Returns when shutdown flag is set or channel closes.
pub fn run_session_manager(
    _bus: EventBus,
    channel: ActorChannel<BusEventEnvelope>,
    shutdown: Arc<AtomicBool>,
    snapshot_interval: Duration,
) {
    let mut state = SessionState::default();

    while !shutdown.load(Ordering::SeqCst) {
        // Check for events (non-blocking)
        if let Ok(event) = channel.rx.try_recv() {
            match event {
                BusEventEnvelope::Domain(domain_event) => {
                    if let Err(e) = state.record_event(&domain_event) {
                        eprintln!("SessionManager: failed to record event: {}", e);
                    }
                }
                BusEventEnvelope::Ephemeral(_) => {
                    // Ephemeral events are not persisted
                }
            }
        }

        // Check if snapshot is needed
        if state.needs_snapshot(snapshot_interval) && state.has_active_session() {
            if let Err(e) = state.flush() {
                eprintln!("SessionManager: failed to snapshot: {}", e);
            } else {
                state.mark_snapshot();
            }
        }

        // Small sleep to avoid busy loop
        std::thread::sleep(Duration::from_millis(10));
    }

    // Cleanup on shutdown
    if let Err(e) = state.close_session() {
        eprintln!("SessionManager: failed to close session on shutdown: {}", e);
    }
}

// ---------------------------------------------------------------------------
// Convenience Functions
// ---------------------------------------------------------------------------

/// List all saved session names
pub fn list_sessions() -> anyhow::Result<Vec<String>> {
    jsonl::list_session_names()
}

/// Load a session by name, returning metadata and events
pub fn load_session(name: &str) -> anyhow::Result<(SessionMeta, Vec<DomainEvent>)> {
    jsonl::load_session(name)
}

/// Delete a session by name
pub fn delete_session(name: &str) -> anyhow::Result<()> {
    jsonl::delete_session(name)
}

/// Get the path for a session
pub fn session_path(name: &str) -> Option<PathBuf> {
    jsonl::session_path(name)
}

/// Start a new session and return the writer
pub fn start_session(
    name: &str,
    provider: &str,
    model: &str,
) -> anyhow::Result<(SessionMeta, jsonl::JsonlWriter)> {
    let path = session_path(name)
        .ok_or_else(|| anyhow::anyhow!("No sessions directory"))?;

    // Ensure directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let meta = SessionMeta::new(name.to_string(), provider.to_string(), model.to_string());
    let writer = jsonl::JsonlWriter::create(&path, &meta)?;

    Ok((meta, writer))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session_jsonl::{JsonlReader, JsonlWriter, SessionMeta};

    fn unique_name(prefix: &str) -> String {
        format!("{}_{}_{}", prefix, std::process::id(), std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos())
    }

    fn cleanup_session(name: &str) {
        if let Some(path) = crate::session_jsonl::session_path(name) {
            let _ = std::fs::remove_file(&path);
        }
    }

    #[test]
    fn test_session_state_start_close() {
        let name = unique_name("start_close");
        let mut state = SessionState::default();

        assert!(!state.has_active_session());
        state.start_session(name.clone(), "test".to_string(), "model".to_string()).unwrap();
        assert!(state.has_active_session());
        assert!(state.is_dirty());

        state.record_event(&DomainEvent::Submit { content: "hello".to_string() }).unwrap();
        assert!(state.is_dirty());

        state.flush().unwrap();
        assert!(!state.is_dirty());

        state.close_session().unwrap();
        assert!(!state.has_active_session());

        cleanup_session(&name);
    }

    #[test]
    fn test_session_state_record_events() {
        let name = unique_name("record_events");
        let mut state = SessionState::default();
        state.start_session(name.clone(), "test".to_string(), "model".to_string()).unwrap();

        let events = vec![
            DomainEvent::Submit { content: "first".to_string() },
            DomainEvent::SpawnAgent,
            DomainEvent::AgentThinking { id: "t1".to_string() },
        ];

        for event in &events {
            state.record_event(event).unwrap();
        }

        let pending = state.take_pending_events();
        assert_eq!(pending.len(), 3);
        assert_eq!(state.take_pending_events().len(), 0);

        state.close_session().unwrap();
        cleanup_session(&name);
    }

    #[test]
    fn test_session_state_snapshot_timing() {
        let mut state = SessionState::default();
        assert!(state.needs_snapshot(Duration::from_secs(60)));

        let name = unique_name("snapshot_timing");
        state.start_session(name, "test".to_string(), "model".to_string()).unwrap();
        assert!(!state.needs_snapshot(Duration::from_secs(60)));

        state.mark_snapshot();
        assert!(!state.needs_snapshot(Duration::from_secs(60)));

        state.close_session().unwrap();
    }

    #[test]
    fn test_session_roundtrip_via_jsonl() {
        let name = unique_name("roundtrip");
        let path = std::env::temp_dir().join(format!("runie_test_{}.jsonl", name));
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::create_dir_all(path.parent().unwrap());

        let meta = SessionMeta::new(name.clone(), "test".to_string(), "model".to_string());

        {
            let mut writer = JsonlWriter::create(&path, &meta).unwrap();
            writer.write_event(&DomainEvent::Submit { content: "hello".to_string() }).unwrap();
            writer.write_event(&DomainEvent::SpawnAgent).unwrap();
            writer.flush().unwrap();
        }

        let (loaded_meta, events) = {
            let mut reader = JsonlReader::open(&path).unwrap();
            reader.read_session().unwrap()
        };

        assert_eq!(loaded_meta.name, name);
        assert_eq!(events.len(), 2);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_delete_session() {
        let name = unique_name("delete_test");
        let real_path = crate::session_jsonl::session_path(&name).unwrap();
        std::fs::create_dir_all(real_path.parent().unwrap()).unwrap();

        let meta = SessionMeta::new(name.clone(), "test".to_string(), "model".to_string());
        {
            let mut writer = JsonlWriter::create(&real_path, &meta).unwrap();
            writer.write_event(&DomainEvent::Submit { content: "test".to_string() }).unwrap();
            writer.flush().unwrap();
        }
        assert!(real_path.exists());

        delete_session(&name).unwrap();
        assert!(!real_path.exists());
    }

    #[test]
    fn test_session_state_resume() {
        let name = unique_name("resume");
        let real_path = crate::session_jsonl::session_path(&name).unwrap();
        std::fs::create_dir_all(real_path.parent().unwrap()).unwrap();

        let meta = SessionMeta::new(name.clone(), "test".to_string(), "model".to_string());
        {
            let mut writer = JsonlWriter::create(&real_path, &meta).unwrap();
            writer.write_event(&DomainEvent::Submit { content: "original".to_string() }).unwrap();
            writer.flush().unwrap();
        }

        let mut state = SessionState::default();
        state.resume_session(name.clone()).unwrap();
        assert!(state.has_active_session());

        state.record_event(&DomainEvent::AgentThinking { id: "t1".to_string() }).unwrap();

        let (_meta, events) = {
            let mut reader = JsonlReader::open(&real_path).unwrap();
            reader.read_session().unwrap()
        };

        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], DomainEvent::Submit { .. }));
        assert!(matches!(events[1], DomainEvent::AgentThinking { .. }));

        state.close_session().unwrap();
        let _ = std::fs::remove_file(&real_path);
    }

    #[test]
    fn test_convenience_session_path() {
        let path = session_path("my_session");
        assert!(path.is_some());
        assert!(path.unwrap().to_string_lossy().contains("my_session.jsonl"));
    }
}
