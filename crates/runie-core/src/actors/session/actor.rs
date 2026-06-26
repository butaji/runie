//! Unified `SessionActor` — owns all durable state.
//!
//! Consolidates the former `PersistenceActor`, `SessionStoreActor`, and
//! `session_actor.rs` into a single actor. This actor owns:
//! - `trust.json` + `history.jsonl` (formerly PersistenceActor)
//! - Named-session CRUD: load/save/delete/import/export/list (formerly SessionStoreActor)
//! - Durable event append + replay + summary index (formerly session_actor.rs)
//! - Session state mutations (messages, tree, pending edits)

use std::path::PathBuf;

use tokio::sync::mpsc;

use crate::actors::{spawn_actor, Actor, ActorHandle, PersistenceActor};
use crate::bus::EventBus;
use crate::event::DurableCoreEvent;
use crate::message::now;
use crate::model::SessionState;
use crate::session::index::{SessionIndex, SessionMetadata};
use crate::session::replay::session_to_durable_events;
use crate::session::store::SessionStore;
use crate::session::Session;
use crate::trust::{TrustDecision, TrustManager};
use crate::Event;

use super::messages::{SessionActorHandle, SessionMsg};

/// Unified session actor owning all durable state.
pub struct SessionActor {
    pub(crate) bus: EventBus<Event>,
    /// Trust manager for trust.json
    #[cfg(test)]
    pub trust: TrustManager,
    #[cfg(not(test))]
    trust: TrustManager,
    /// Session store for named sessions and durable events
    #[cfg(test)]
    pub store: SessionStore,
    #[cfg(not(test))]
    store: SessionStore,
    /// Current session id for durable appends
    #[cfg(test)]
    pub session_id: String,
    #[cfg(not(test))]
    session_id: String,
    /// Display name for current session
    #[cfg(test)]
    pub display_name: String,
    #[cfg(not(test))]
    display_name: String,
    /// Message count for summary generation
    #[cfg(test)]
    pub message_count: usize,
    #[cfg(not(test))]
    message_count: usize,
    /// Summary buffer (first 500 chars of content)
    #[cfg(test)]
    pub summary_buffer: String,
    #[cfg(not(test))]
    summary_buffer: String,
    /// Started timestamp
    #[cfg(test)]
    pub started_at: f64,
    #[cfg(not(test))]
    started_at: f64,
    /// Authoritative session state (messages, tree, pending edits)
    pub(crate) session_state: SessionState,
    /// Next message id counter
    pub(crate) next_id: usize,
    /// Thought sequence counter (reserved for future use)
    #[allow(dead_code)]
    #[cfg(test)]
    pub thought_seq: usize,
    #[cfg(not(test))]
    #[allow(dead_code)]
    thought_seq: usize,
}

impl SessionActor {
    /// Spawn a `SessionActor` on the given event bus.
    pub fn spawn(bus: EventBus<Event>) -> (SessionActorHandle, ActorHandle) {
        let store = SessionStore::default_store()
            .unwrap_or_else(|| SessionStore::new(std::env::temp_dir().join("runie_sessions")));
        let actor = Self {
            bus: bus.clone(),
            trust: TrustManager::default(),
            store,
            session_id: String::new(),
            display_name: String::new(),
            message_count: 0,
            summary_buffer: String::new(),
            started_at: now(),
            session_state: SessionState::default(),
            next_id: 0,
            thought_seq: 0,
        };
        let (tx, handle) = spawn_actor(actor, bus);
        (SessionActorHandle::new(tx), handle)
    }
}

impl Actor for SessionActor {
    type Msg = SessionMsg;
    type Event = Event;

    async fn run_body(mut self, mut rx: mpsc::Receiver<Self::Msg>, bus: EventBus<Event>) {
        self.load_all(&bus).await;
        while let Some(msg) = rx.recv().await {
            self.handle_msg(msg).await;
        }
    }
}

impl PersistenceActor for SessionActor {
    async fn load_all(&mut self, bus: &EventBus<Event>) {
        let trust = tokio::task::spawn_blocking(TrustManager::load)
            .await
            .unwrap_or_default();
        bus.publish(Event::TrustLoaded {
            decisions: trust.decisions(),
        });

        let entries = tokio::task::spawn_blocking(crate::input_history::load_history)
            .await
            .ok()
            .and_then(|r| r.ok())
            .unwrap_or_default();
        bus.publish(Event::HistoryLoaded { entries });
    }
}

impl SessionActor {
    /// Dispatch incoming messages.
    async fn handle_msg(&mut self, msg: SessionMsg) {
        match msg {
            SessionMsg::SetTrust { path, decision } => self.set_trust(path, decision).await,
            SessionMsg::AppendHistory { entry } => self.append_history(entry).await,
            SessionMsg::Load { name } => self.load_session(name).await,
            SessionMsg::Save { name, session } => self.save_session(name, session).await,
            SessionMsg::Delete { name } => self.delete_session(name).await,
            SessionMsg::Import { path } => self.import_session(path).await,
            SessionMsg::Export { path, session } => self.export_session(path, session).await,
            SessionMsg::List => self.list_sessions().await,
            SessionMsg::AddUserMessage { content, images } => {
                self.handle_add_user_message(content, images)
            }
            SessionMsg::AddSystemMessage { content } => self.handle_add_system_message(content),
            SessionMsg::AddToolMessage { id, name, content } => {
                self.handle_add_tool_message(id, name, content)
            }
            SessionMsg::UpdateToolMessage { id_contains, content } => {
                self.handle_update_tool_message(&id_contains, &content)
            }
            SessionMsg::AddTurnComplete { id, content } => {
                self.handle_add_turn_complete(id, content)
            }
            SessionMsg::AddErrorMessage { id, content } => {
                self.handle_add_error_message(id, content)
            }
            SessionMsg::Reset => self.handle_reset(),
            SessionMsg::ForkAt { index } => self.handle_fork_at(index),
            SessionMsg::CloneBranch => self.handle_clone_branch(),
            SessionMsg::PushPendingEdit { edit } => self.handle_push_pending_edit(edit),
            SessionMsg::DrainPendingEdits => self.handle_drain_pending_edits(),
            SessionMsg::ClearPendingEdits => self.handle_clear_pending_edits(),
        }
    }

    // ── Trust + history ────────────────────────────────────────────────────────

    async fn set_trust(&mut self, path: PathBuf, decision: TrustDecision) {
        self.trust.set(&path, decision);
        let trust = self.trust.clone();
        let path_clone = path.clone();
        let decision_clone = decision;
        let _ = tokio::task::spawn_blocking(move || trust.save()).await;
        self.bus.publish(Event::TrustChanged {
            path: path_clone,
            decision: decision_clone,
        });
    }

    async fn append_history(&self, entry: String) {
        let entry_clone = entry;
        let _ =
            tokio::task::spawn_blocking(move || crate::input_history::append_history(&entry_clone))
                .await;
    }

    // ── Session CRUD ────────────────────────────────────────────────────────────

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
                format!(
                    "Session '{}' not found. Use /sessions to list saved sessions.",
                    name
                ),
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
                format!(
                    "Session '{}' not found. Use /sessions to list saved sessions.",
                    name
                ),
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

    // ── Metadata helpers ───────────────────────────────────────────────────────

    fn build_metadata_from_session(&self, session: &Session, name: &str) -> SessionMetadata {
        SessionMetadata {
            id: name.to_owned(),
            display_name: session
                .display_name
                .clone()
                .unwrap_or_else(|| name.to_owned()),
            created_at: session.created_at,
            updated_at: now(),
            message_count: session.messages.len(),
            summary: None,
            is_starred: false,
            is_system: false,
        }
    }

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
    async fn persist(&self, durable: &DurableCoreEvent) -> anyhow::Result<()> {
        let store = self.store.clone();
        let session_id = self.session_id.clone();
        let event = durable.clone();
        tokio::task::spawn_blocking(move || store.append(&session_id, &event))
            .await
            .map_err(|e| anyhow::anyhow!("spawn_blocking failed: {}", e))?
    }

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

    // ── Helpers ────────────────────────────────────────────────────────────────

    fn emit(&self, event: Event) {
        let _ = self.bus.publish(event);
    }

    fn fail(&self, operation: &str, error: String) {
        self.emit(Event::SessionOperationFailed {
            operation: operation.to_owned(),
            error,
        });
    }
}
