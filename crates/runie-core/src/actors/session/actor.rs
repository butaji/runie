//! Unified `SessionActor` — owns all durable state.
//!
//! Consolidates the former `PersistenceActor`, `SessionStoreActor`, and
//! `session_actor.rs` into a single actor. This actor owns:
//! - `trust.json` + `history.jsonl` (formerly PersistenceActor)
//! - Named-session CRUD: load/save/delete/import/export/list (formerly SessionStoreActor)
//! - Durable event append + replay + summary index (formerly session_actor.rs)

use std::path::PathBuf;

use tokio::sync::mpsc;

use crate::actors::{spawn_actor, Actor, ActorHandle};
use crate::bus::EventBus;
use crate::event::{DurableCoreEvent, Event};
use crate::message::now;
use crate::session::Session;
use crate::session::index::{SessionIndex, SessionMetadata};
use crate::session::replay::session_to_durable_events;
use crate::session::store::SessionStore;
use crate::trust::{TrustDecision, TrustManager};

use super::messages::{SessionActorHandle, SessionMsg};

/// Unified session actor owning all durable state.
#[allow(dead_code)]
pub struct SessionActor {
    bus: EventBus<Event>,
    /// Trust manager for trust.json
    trust: TrustManager,
    /// Session store for named sessions and durable events
    store: SessionStore,
    /// Current session id for durable appends
    session_id: String,
    /// Display name for current session
    display_name: String,
    /// Message count for summary generation
    message_count: usize,
    /// Summary buffer (first 500 chars of content)
    summary_buffer: String,
    /// Started timestamp
    started_at: f64,
}

impl SessionActor {
    /// Spawn a `SessionActor` on the given event bus.
    pub fn spawn(bus: EventBus<Event>) -> (SessionActorHandle, ActorHandle) {
        let store = SessionStore::default_store().unwrap_or_else(|| {
            SessionStore::new(std::env::temp_dir().join("runie_sessions"))
        });
        let actor = Self {
            bus: bus.clone(),
            trust: TrustManager::default(),
            store,
            session_id: String::new(),
            display_name: String::new(),
            message_count: 0,
            summary_buffer: String::new(),
            started_at: now(),
        };
        let (tx, handle) = spawn_actor(actor, bus);
        (SessionActorHandle::new(tx), handle)
    }
}

impl Actor for SessionActor {
    type Msg = SessionMsg;
    type Event = Event;

    async fn run_body(mut self, mut rx: mpsc::Receiver<Self::Msg>, _bus: EventBus<Event>) {
        // Load trust + history on startup
        self.load_all().await;
        // Main message loop
        while let Some(msg) = rx.recv().await {
            self.handle_msg(msg).await;
        }
    }
}

impl SessionActor {
    /// Load trust and history from disk on startup.
    async fn load_all(&self) {
        // Load trust
        let trust = tokio::task::spawn_blocking(TrustManager::load)
            .await
            .unwrap_or_default();
        let decisions = trust.decisions();
        self.bus.publish(Event::TrustLoaded { decisions });

        // Load history
        let entries = tokio::task::spawn_blocking(crate::input_history::load_history)
            .await
            .ok()
            .and_then(|r| r.ok())
            .unwrap_or_default();
        self.bus.publish(Event::HistoryLoaded { entries });
    }

    /// Dispatch incoming messages.
    async fn handle_msg(&mut self, msg: SessionMsg) {
        match msg {
            // Trust + history
            SessionMsg::SetTrust { path, decision } => self.set_trust(path, decision).await,
            SessionMsg::AppendHistory { entry } => self.append_history(entry).await,
            // Session CRUD
            SessionMsg::Load { name } => self.load_session(name).await,
            SessionMsg::Save { name, session } => self.save_session(name, session).await,
            SessionMsg::Delete { name } => self.delete_session(name).await,
            SessionMsg::Import { path } => self.import_session(path).await,
            SessionMsg::Export { path, session } => self.export_session(path, session).await,
            SessionMsg::List => self.list_sessions().await,
        }
    }

    // -------------------------------------------------------------------------
    // Trust + history handlers (formerly PersistenceActor)
    // -------------------------------------------------------------------------

    async fn set_trust(&mut self, path: PathBuf, decision: TrustDecision) {
        self.trust.set(&path, decision);
        let trust = self.trust.clone();
        let path_clone = path.clone();
        let decision_clone = decision.clone();
        let _ = tokio::task::spawn_blocking(move || trust.save()).await;
        self.bus.publish(Event::TrustChanged {
            path: path_clone,
            decision: decision_clone,
        });
    }

    async fn append_history(&self, entry: String) {
        let entry_clone = entry.clone();
        let _ = tokio::task::spawn_blocking(move || crate::input_history::append_history(&entry_clone))
            .await;
    }

    // -------------------------------------------------------------------------
    // Session CRUD handlers (formerly SessionStoreActor)
    // -------------------------------------------------------------------------

    async fn load_session(&self, name: String) {
        let store = self.store.clone();
        let name_for_task = name.clone();
        let res = tokio::task::spawn_blocking(move || -> anyhow::Result<_> {
            let events = store.load_events(&name_for_task)?;
            if events.is_empty() {
                return Err(anyhow::anyhow!("not found"));
            }
            let data_dir = store.dir().parent().unwrap_or(store.dir()).to_path_buf();
            let metadata = SessionIndex::load(&data_dir)
                .ok()
                .and_then(|idx| idx.get(&name_for_task).cloned());
            Ok((events, metadata))
        })
        .await;

        match res {
            Ok(Ok((events, metadata))) => {
                self.emit(Event::SessionLoaded {
                    name,
                    events: Box::new(events),
                    metadata: metadata.map(Box::new),
                });
            }
            _ => self.fail(
                "load",
                format!("Session '{}' not found. Use /sessions to list saved sessions.", name),
            ),
        }
    }

    async fn save_session(&self, name: String, session: Session) {
        let store = self.store.clone();
        let name_for_task = name.clone();
        let events = session_to_durable_events(&session);
        let meta = self.build_metadata_from_session(&session, &name);

        let res = tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            store.append_batch(&name_for_task, &events)?;
            store.update_index(&meta)?;
            Ok(())
        })
        .await;

        match res {
            Ok(Ok(())) => self.emit(Event::SessionSaved { name }),
            Ok(Err(e)) => self.fail("save", e.to_string()),
            Err(e) => self.fail("save", e.to_string()),
        }
    }

    async fn delete_session(&self, name: String) {
        let store = self.store.clone();
        let name_for_task = name.clone();
        let res = tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            store.delete(&name_for_task)?;
            Ok(())
        })
        .await;

        match res {
            Ok(Ok(())) => self.emit(Event::SessionDeleted { name }),
            Ok(Err(_)) => self.fail(
                "delete",
                format!("Session '{}' not found. Use /sessions to list saved sessions.", name),
            ),
            Err(e) => self.fail("delete", e.to_string()),
        }
    }

    async fn import_session(&self, path: PathBuf) {
        let path_for_task = path.clone();
        let read_res =
            tokio::task::spawn_blocking(move || std::fs::read_to_string(&path_for_task)).await;

        let json = match read_res {
            Ok(Ok(json)) => json,
            Ok(Err(e)) => return self.fail("import", e.to_string()),
            Err(e) => return self.fail("import", e.to_string()),
        };

        match serde_json::from_str::<Session>(&json) {
            Ok(session) => self.emit(Event::SessionImported {
                session: Box::new(session),
            }),
            Err(e) => self.fail("import", e.to_string()),
        }
    }

    async fn export_session(&self, path: PathBuf, session: Session) {
        let display = path.to_string_lossy().to_string();
        let json = match serde_json::to_string_pretty(&session) {
            Ok(json) => json,
            Err(e) => return self.fail("export", e.to_string()),
        };

        let res = tokio::task::spawn_blocking(move || std::fs::write(&path, json)).await;
        match res {
            Ok(Ok(())) => self.emit(Event::SessionExported { path: display }),
            Ok(Err(e)) => self.fail("export", e.to_string()),
            Err(e) => self.fail("export", e.to_string()),
        }
    }

    async fn list_sessions(&self) {
        let store = self.store.clone();
        let res = tokio::task::spawn_blocking(move || store.list()).await;

        match res {
            Ok(Ok(sessions)) => self.emit(Event::SessionList {
                sessions: Box::new(sessions),
            }),
            Ok(Err(e)) => self.fail("list", e.to_string()),
            Err(e) => self.fail("list", e.to_string()),
        }
    }

    // -------------------------------------------------------------------------
    // Durable event handling (formerly session_actor.rs)
    // -------------------------------------------------------------------------

    /// Build metadata from a session.
    fn build_metadata_from_session(&self, session: &Session, name: &str) -> SessionMetadata {
        SessionMetadata {
            id: name.to_string(),
            display_name: session
                .display_name
                .clone()
                .unwrap_or_else(|| name.to_string()),
            created_at: session.created_at,
            updated_at: now(),
            message_count: session.messages.len(),
            summary: None,
            is_starred: false,
            is_system: false,
        }
    }

    /// Build metadata for current session (durable appends).
    #[allow(dead_code)]
    fn build_meta(&self) -> SessionMetadata {
        SessionMetadata {
            id: self.session_id.clone(),
            display_name: self.display_name.clone(),
            created_at: self.started_at,
            updated_at: now(),
            message_count: self.message_count,
            summary: Some(self.generate_summary()),
            is_starred: false,
            is_system: false,
        }
    }

    /// Generate summary from first 500 chars of content.
    #[allow(dead_code)]
    fn generate_summary(&self) -> String {
        let chars: Vec<char> = self.summary_buffer.chars().take(500).collect();
        let truncated: String = chars.into_iter().collect();
        if truncated.len() < self.summary_buffer.len() {
            format!("{}…", truncated)
        } else {
            truncated
        }
    }

    #[allow(dead_code)]
    fn append_to_summary(&mut self, msg: &str) {
        if self.summary_buffer.len() < 500 {
            self.summary_buffer.push_str(msg);
        }
    }

    /// Persist a durable event to the session file.
    #[allow(dead_code)]
    async fn persist(&self, durable: &DurableCoreEvent) -> anyhow::Result<()> {
        let store = self.store.clone();
        let session_id = self.session_id.clone();
        let event = durable.clone();
        tokio::task::spawn_blocking(move || store.append(&session_id, &event))
            .await
            .map_err(|e| anyhow::anyhow!("spawn_blocking failed: {}", e))?
    }

    /// Update the session index.
    #[allow(dead_code)]
    async fn update_index(&self) {
        let store = self.store.clone();
        let meta = self.build_meta();
        if let Err(e) = tokio::task::spawn_blocking(move || store.update_index(&meta))
            .await
            .map_err(|e| anyhow::anyhow!("spawn_blocking failed: {}", e))
            .and_then(|r| r)
        {
            eprintln!("SessionActor: failed to update index: {}", e);
        }
    }

    // -------------------------------------------------------------------------
    // Helpers
    // -------------------------------------------------------------------------

    fn emit(&self, event: Event) {
        let _ = self.bus.publish(event);
    }

    fn fail(&self, operation: &str, error: String) {
        self.emit(Event::SessionOperationFailed {
            operation: operation.to_string(),
            error,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_store() -> (SessionStore, TempDir) {
        let dir = TempDir::new().unwrap();
        let store = SessionStore::new(dir.path().to_path_buf());
        (store, dir)
    }

    #[tokio::test]
    async fn actor_loads_and_emits_trust_and_history() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg = tmp.path().join("cfg");
        let data = tmp.path().join("data");
        std::fs::create_dir_all(&cfg).unwrap();
        std::fs::create_dir_all(&data).unwrap();
        std::env::set_var("RUNIE_TEST_CONFIG_DIR", &cfg);
        std::env::set_var("RUNIE_TEST_DATA_DIR", &data);

        let bus = EventBus::<Event>::new(4);
        let mut sub = bus.subscribe();
        let (handle, _actor_handle) = SessionActor::spawn(bus);

        let mut saw_trust = false;
        let mut saw_history = false;
        for _ in 0..60 {
            if saw_trust && saw_history {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            while let Ok(evt) = sub.try_recv() {
                match evt {
                    Event::TrustLoaded { .. } => saw_trust = true,
                    Event::HistoryLoaded { .. } => saw_history = true,
                    _ => {}
                }
            }
        }
        assert!(saw_trust);
        assert!(saw_history);

        handle.set_trust(PathBuf::from("/tmp/project"), TrustDecision::Trusted).await;

        let mut saw_changed = false;
        for _ in 0..60 {
            if saw_changed {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            while let Ok(evt) = sub.try_recv() {
                if matches!(evt, Event::TrustChanged { .. }) {
                    saw_changed = true;
                }
            }
        }
        assert!(saw_changed);

        std::env::remove_var("RUNIE_TEST_CONFIG_DIR");
        std::env::remove_var("RUNIE_TEST_DATA_DIR");
    }

    #[tokio::test]
    async fn session_metadata_built_once() {
        // Verify that build_meta appears exactly once in the unified actor
        // This is tested by the fact that we only have one build_meta implementation
        let (store, _dir) = make_store();
        let actor = SessionActor {
            bus: EventBus::new(4),
            trust: TrustManager::default(),
            store,
            session_id: "test".into(),
            display_name: "Test".into(),
            message_count: 5,
            summary_buffer: "Hello world".into(),
            started_at: 1000.0,
        };

        let meta = actor.build_meta();
        assert_eq!(meta.id, "test");
        assert_eq!(meta.display_name, "Test");
        assert_eq!(meta.message_count, 5);
        assert!(meta.summary.is_some());
    }
}
