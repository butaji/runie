//! `SessionActor` — owns session state, trust, and session store.
//!
//! No ractor dependency. Actor is a tokio task with mpsc channel.

use camino::Utf8PathBuf;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tracing::instrument;

use crate::bus::EventBus;
use crate::edit_preview::EditPreview;
use crate::message::{now, MessageOrigin, Part};
use crate::model::{ChatMessage, Role, SessionState};
use crate::proto::message::MessageMetadata;
use crate::session::replay::session_to_durable_events;
use crate::session::store::SessionStore;
use crate::session::tree::SessionTree;
use crate::session::{Session, SessionMetadata};
use crate::trust::{TrustDecision, TrustManager};
use crate::Event;

use super::messages::SessionMsg;

// ── Handle ────────────────────────────────────────────────────────────────────

/// SessionActor handle — cloneable, fire-and-forget sender.
#[derive(Clone, Debug)]
pub struct SessionHandle {
    tx: mpsc::UnboundedSender<SessionMsg>,
}

impl SessionHandle {
    /// Create a new handle wrapping a sender.
    pub fn new(tx: mpsc::UnboundedSender<SessionMsg>) -> Self {
        Self { tx }
    }

    /// Send a message to the actor (fire-and-forget).
    pub async fn send(&self, msg: SessionMsg) {
        let _ = self.tx.send(msg);
    }

    /// Request a trust decision change.
    pub async fn set_trust(&self, path: PathBuf, decision: TrustDecision) {
        let path_utf8 = Utf8PathBuf::from_path_buf(path).unwrap_or_else(|_| Utf8PathBuf::from("."));
        let _ = self.tx.send(SessionMsg::SetTrust { path: path_utf8, decision });
    }

    /// Append an entry to the history file.
    pub async fn append_history(&self, entry: String) {
        let _ = self.tx.send(SessionMsg::AppendHistory { entry });
    }

    /// Append an entry to the history file (sync fire-and-forget).
    pub fn try_append_history(&self, entry: String) {
        let _ = self.tx.send(SessionMsg::AppendHistory { entry });
    }

    /// Request loading a named session.
    pub async fn load(&self, name: String) {
        let _ = self.tx.send(SessionMsg::Load { name });
    }

    /// Request saving a session snapshot.
    pub async fn save(&self, name: String, session: Session) {
        let _ = self.tx.send(SessionMsg::Save { name, session });
    }

    /// Request deleting a named session.
    pub async fn delete(&self, name: String) {
        let _ = self.tx.send(SessionMsg::Delete { name });
    }

    /// Request listing saved sessions.
    pub async fn list(&self) {
        let _ = self.tx.send(SessionMsg::List);
    }

    /// Request resuming the most recent session.
    pub async fn resume_most_recent(&self) {
        let _ = self.tx.send(SessionMsg::ResumeMostRecent);
    }

    /// Try to request resuming the most recent session (fire-and-forget).
    pub fn try_resume_most_recent(&self) {
        let _ = self.tx.send(SessionMsg::ResumeMostRecent);
    }

    /// Try to add a user message (fire-and-forget).
    pub fn try_add_user_message(&self, content: String, images: Vec<String>) {
        let _ = self.tx.send(SessionMsg::AddUserMessage { content, images });
    }

    /// Try to add a system message (fire-and-forget).
    pub fn try_add_system_message(&self, content: String) {
        let _ = self.tx.send(SessionMsg::AddSystemMessage { content });
    }

    /// Try to add a tool message (fire-and-forget).
    pub fn try_add_tool_message(&self, id: String, name: String, content: String) {
        let _ = self.tx.send(SessionMsg::AddToolMessage { id, name, content });
    }

    /// Try to update a tool message (fire-and-forget).
    pub fn try_update_tool_message(&self, id_contains: String, content: String) {
        let _ = self.tx.send(SessionMsg::UpdateToolMessage { id_contains, content });
    }

    /// Try to add a turn-complete message (fire-and-forget).
    pub fn try_add_turn_complete(&self, id: String, content: String) {
        let _ = self.tx.send(SessionMsg::AddTurnComplete { id, content });
    }

    /// Try to add an error message (fire-and-forget).
    pub fn try_add_error_message(&self, id: String, content: String) {
        let _ = self.tx.send(SessionMsg::AddErrorMessage { id, content });
    }

    /// Try to reset session (fire-and-forget).
    pub fn try_reset(&self) {
        let _ = self.tx.send(SessionMsg::Reset);
    }

    /// Try to fork at message index (fire-and-forget).
    pub fn try_fork_at(&self, index: usize) {
        let _ = self.tx.send(SessionMsg::ForkAt { index });
    }

    /// Try to clone branch (fire-and-forget).
    pub fn try_clone_branch(&self) {
        let _ = self.tx.send(SessionMsg::CloneBranch);
    }

    /// Try to push a pending edit (fire-and-forget).
    pub fn try_push_pending_edit(&self, edit: EditPreview) {
        let _ = self.tx.send(SessionMsg::PushPendingEdit { edit });
    }

    /// Try to drain pending edits (fire-and-forget).
    pub fn try_drain_pending_edits(&self) {
        let _ = self.tx.send(SessionMsg::DrainPendingEdits);
    }

    /// Try to clear pending edits (fire-and-forget).
    pub fn try_clear_pending_edits(&self) {
        let _ = self.tx.send(SessionMsg::ClearPendingEdits);
    }

    /// Request importing a session from a file path.
    pub async fn import(&self, path: PathBuf) {
        let _ = self.tx.send(SessionMsg::Import { path });
    }

    /// Request exporting a session to a file path.
    pub async fn export(&self, path: PathBuf, session: Session) {
        let _ = self.tx.send(SessionMsg::Export { path, session });
    }

    /// Try to send a message (non-blocking).
    pub fn try_send(&self, msg: SessionMsg) -> Result<(), mpsc::error::SendError<SessionMsg>> {
        self.tx.send(msg)
    }
}

// Backward-compat aliases
#[allow(unused_imports)]
pub use SessionHandle as RactorSessionHandle;

// ── Actor state ───────────────────────────────────────────────────────────────

/// Mutable state owned by SessionActor.
/// EventBus is Clone and publish takes &self, no Mutex needed.
pub struct SessionActorState {
    pub bus: EventBus<Event>,
    pub trust: TrustManager,
    pub store: SessionStore,
    pub session_state: SessionState,
    pub next_id: usize,
}

impl SessionActorState {
    pub fn emit(&self, event: Event) {
        self.bus.publish(event);
    }

    pub fn emit_changed(&self) {
        let state = self.session_state.clone();
        self.emit(Event::SessionChanged { state: Box::new(state) });
    }
}

/// Bump the session updated timestamp.
fn bump_time(state: &mut SessionState) {
    state.session_updated_at = now();
}

// ── Handler functions ─────────────────────────────────────────────────────────

impl SessionActorState {
    /// Handle add user message.
    pub fn handle_add_user_message(&mut self, content: String, images: Vec<String>) {
        let id = {
            let id = format!("req.{}", self.next_id);
            self.next_id += 1;
            id
        };
        self.session_state.image_attachments.extend(images);
        self.session_state.messages.push(ChatMessage {
            role: Role::User,
            timestamp: now(),
            id: id.clone(),
            parts: vec![Part::Text { content: content.clone() }],
            metadata: MessageMetadata { origin: MessageOrigin::User, ..Default::default() },
            ..Default::default()
        });
        bump_time(&mut self.session_state);
        self.emit(Event::SessionMessageAdded { id, role: "user".into(), content });
        self.emit_changed();
    }

    /// Handle add system message.
    pub fn handle_add_system_message(&mut self, content: String) {
        self.session_state.messages.push(ChatMessage {
            role: Role::System,
            timestamp: now(),
            id: "system".to_owned(),
            parts: vec![Part::Text { content }],
            metadata: MessageMetadata { origin: MessageOrigin::System, ..Default::default() },
            ..Default::default()
        });
        bump_time(&mut self.session_state);
        self.emit_changed();
    }

    /// Handle add tool message.
    pub fn handle_add_tool_message(&mut self, id: String, name: String, content: String) {
        self.session_state.messages.push(ChatMessage {
            role: Role::Tool,
            timestamp: now(),
            id: id.clone(),
            parts: vec![Part::Text { content: content.clone() }],
            tool_call_id: Some(name),
            metadata: MessageMetadata { origin: MessageOrigin::Tool, ..Default::default() },
            ..Default::default()
        });
        bump_time(&mut self.session_state);
        self.emit(Event::SessionMessageAdded { id, role: "tool".into(), content });
        self.emit_changed();
    }

    /// Handle update tool message.
    pub fn handle_update_tool_message(&mut self, id_contains: &str, content: &str) {
        if let Some(idx) = self
            .session_state
            .messages
            .iter()
            .rposition(|m| m.role == Role::Tool && m.id.contains(id_contains))
        {
            if let Some(msg) = self.session_state.messages.get_mut(idx) {
                msg.set_text_part(content.to_owned());
                msg.timestamp = now();
            }
        }
        bump_time(&mut self.session_state);
        self.emit_changed();
    }

    /// Handle add turn complete message.
    pub fn handle_add_turn_complete(&mut self, id: String, content: String) {
        if let Some(idx) = self
            .session_state
            .messages
            .iter()
            .position(|m| m.role == Role::TurnComplete && m.id == id)
        {
            if let Some(msg) = self.session_state.messages.get_mut(idx) {
                msg.set_text_part(content);
                msg.timestamp = now();
            }
        } else {
            self.session_state.messages.push(ChatMessage {
                role: Role::TurnComplete,
                timestamp: now(),
                id,
                parts: vec![Part::Text { content }],
                ..Default::default()
            });
        }
        bump_time(&mut self.session_state);
        self.emit_changed();
    }

    /// Handle add error message.
    pub fn handle_add_error_message(&mut self, id: String, content: String) {
        self.session_state.messages.push(ChatMessage {
            role: Role::Assistant,
            timestamp: now(),
            id: format!("error.{}", id),
            parts: vec![Part::Text { content }],
            ..Default::default()
        });
        bump_time(&mut self.session_state);
        self.emit_changed();
    }

    /// Handle reset session state.
    pub fn handle_reset(&mut self) {
        self.session_state = SessionState::default();
        self.emit_changed();
    }

    /// Handle fork session at index.
    pub fn handle_fork_at(&mut self, index: usize) {
        match self.session_state.session_tree.as_mut() {
            Some(tree) => {
                if let Some(path) = tree.fork_at(index) {
                    tree.navigate_to(&path);
                }
            }
            None => {
                let tree = SessionTree::from_messages(&self.session_state.messages);
                let mut new_tree = tree;
                if let Some(path) = new_tree.fork_at(index) {
                    new_tree.navigate_to(&path);
                }
                self.session_state.session_tree = Some(new_tree);
            }
        }
        bump_time(&mut self.session_state);
        self.emit_changed();
    }

    /// Handle clone branch.
    pub fn handle_clone_branch(&mut self) {
        let tree = self
            .session_state
            .session_tree
            .clone()
            .unwrap_or_else(|| SessionTree::from_messages(&self.session_state.messages));
        self.session_state.session_tree = Some(tree);
        bump_time(&mut self.session_state);
        self.emit_changed();
    }

    /// Handle push pending edit.
    pub fn handle_push_pending_edit(&mut self, edit: EditPreview) {
        self.session_state.pending_edits.push(edit);
        self.emit_changed();
    }

    /// Handle drain pending edits.
    pub fn handle_drain_pending_edits(&mut self) {
        self.session_state.pending_edits.clear();
        self.emit_changed();
    }

    /// Handle clear pending edits.
    pub fn handle_clear_pending_edits(&mut self) {
        self.session_state.pending_edits.clear();
        self.emit_changed();
    }

    /// Handle set trust decision.
    pub async fn handle_set_trust(&mut self, path: Utf8PathBuf, decision: TrustDecision) {
        self.trust.set(&path, decision);
        let trust = self.trust.clone();
        let path_clone = path.clone();
        let decision_clone = decision;
        let _ = tokio::task::spawn_blocking(move || trust.save()).await;
        self.emit(Event::TrustChanged { path: path_clone, decision: decision_clone });
    }

    /// Handle append to history.
    pub async fn handle_append_history(&mut self, entry: String) {
        let entry_clone = entry;
        let _ = tokio::task::spawn_blocking(move || crate::input_history::append_history(&entry_clone)).await;
    }

    /// Handle load session.
    pub async fn handle_load(&mut self, name: String) {
        let store = self.store.clone();
        let name_for_task = name.clone();
        let res = tokio::task::spawn_blocking(move || -> anyhow::Result<_> {
            let events = store.load_events(&name_for_task)?;
            if events.is_empty() {
                return Err(anyhow::anyhow!("not found"));
            }
            let metadata = store.load_metadata(&name_for_task).ok().flatten();
            Ok((events, metadata))
        })
        .await;

        match res {
            Ok(Ok((events, metadata))) => {
                self.emit(Event::SessionLoaded { name, events: Box::new(events), metadata: metadata.map(Box::new) });
            }
            _ => {
                self.emit(Event::SessionOperationFailed {
                    operation: "load".to_owned(),
                    error: format!("Session '{}' not found. Use /sessions to list saved sessions.", name),
                });
            }
        }
    }

    /// Handle save session.
    pub async fn handle_save(&mut self, name: String, session: Session) {
        let store = self.store.clone();
        let name_for_task = name.clone();
        let events = session_to_durable_events(&session);
        let meta = SessionMetadata {
            id: name_for_task.clone(),
            display_name: session.display_name.clone().unwrap_or_else(|| name_for_task.clone()),
            created_at: session.created_at,
            updated_at: now(),
            message_count: session.messages.len(),
            summary: None,
            is_starred: false,
            is_system: false,
            active_plan_id: None,
        };

        let res = tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            store.append_batch(&name_for_task, &events)?;
            store.update_metadata(&meta)?;
            Ok(())
        })
        .await;

        match res {
            Ok(Ok(())) => self.emit(Event::SessionSaved { name }),
            Ok(Err(e)) => {
                self.emit(Event::SessionOperationFailed { operation: "save".to_owned(), error: e.to_string() })
            }
            Err(e) => {
                self.emit(Event::SessionOperationFailed { operation: "save".to_owned(), error: e.to_string() })
            }
        }
    }

    /// Handle delete session.
    pub async fn handle_delete(&mut self, name: String) {
        let store = self.store.clone();
        let name_for_task = name.clone();
        let res = tokio::task::spawn_blocking(move || store.delete(&name_for_task)).await;

        match res {
            Ok(Ok(())) => self.emit(Event::SessionDeleted { name }),
            Ok(Err(_)) => {
                self.emit(Event::SessionOperationFailed {
                    operation: "delete".to_owned(),
                    error: format!("Session '{}' not found. Use /sessions to list saved sessions.", name),
                });
            }
            Err(e) => {
                self.emit(Event::SessionOperationFailed { operation: "delete".to_owned(), error: e.to_string() })
            }
        }
    }

    /// Handle list sessions.
    pub async fn handle_list(&mut self) {
        let store = self.store.clone();
        let res = tokio::task::spawn_blocking(move || store.list()).await;

        match res {
            Ok(Ok(sessions)) => self.emit(Event::SessionList { sessions: Box::new(sessions) }),
            Ok(Err(e)) => {
                self.emit(Event::SessionOperationFailed { operation: "list".to_owned(), error: e.to_string() })
            }
            Err(e) => {
                self.emit(Event::SessionOperationFailed { operation: "list".to_owned(), error: e.to_string() })
            }
        }
    }

    /// Handle resume most recent session.
    pub async fn handle_resume_most_recent(&mut self) {
        let store = self.store.clone();
        let res = tokio::task::spawn_blocking(move || {
            let names = store.list().ok()?;
            let mut most_recent = None;
            let mut most_recent_time = 0.0f64;
            for name in names {
                if let Ok(Some(meta)) = store.load_metadata(&name) {
                    if meta.updated_at > most_recent_time {
                        most_recent_time = meta.updated_at;
                        most_recent = Some(name);
                    }
                }
            }
            most_recent
        })
        .await;

        match res {
            Ok(Some(name)) => self.handle_load(name).await,
            Ok(None) => {
                self.emit(Event::SessionOperationFailed {
                    operation: "resume".to_owned(),
                    error: "No sessions to resume.".to_owned(),
                });
            }
            Err(e) => {
                self.emit(Event::SessionOperationFailed { operation: "resume".to_owned(), error: e.to_string() });
            }
        }
    }

    /// Handle import session.
    pub async fn handle_import(&mut self, path: PathBuf) {
        let path_clone = path.clone();
        match tokio::task::spawn_blocking(move || std::fs::read_to_string(&path_clone)).await {
            Ok(Ok(json)) => match serde_json::from_str::<Session>(&json) {
                Ok(session) => self.emit(Event::SessionImported { session: Box::new(session) }),
                Err(e) => {
                    self.emit(Event::SessionOperationFailed { operation: "import".to_owned(), error: e.to_string() })
                }
            },
            Ok(Err(e)) => {
                self.emit(Event::SessionOperationFailed { operation: "import".to_owned(), error: e.to_string() })
            }
            Err(e) => {
                self.emit(Event::SessionOperationFailed { operation: "import".to_owned(), error: e.to_string() })
            }
        }
    }

    /// Handle export session.
    pub async fn handle_export(&mut self, path: PathBuf, session: Session) {
        let path_clone = path.clone();
        let json = match serde_json::to_string_pretty(&session) {
            Ok(json) => json,
            Err(e) => {
                self.emit(Event::SessionOperationFailed { operation: "export".to_owned(), error: e.to_string() });
                return;
            }
        };
        match tokio::task::spawn_blocking(move || std::fs::write(&path, json)).await {
            Ok(Ok(())) => {
                self.emit(Event::SessionExported { path: path_clone.to_string_lossy().to_string() })
            }
            Ok(Err(e)) => {
                self.emit(Event::SessionOperationFailed { operation: "export".to_owned(), error: e.to_string() })
            }
            Err(e) => {
                self.emit(Event::SessionOperationFailed { operation: "export".to_owned(), error: e.to_string() })
            }
        }
    }
}

// ── Actor struct ─────────────────────────────────────────────────────────────

/// The SessionActor — processes session messages.
struct SessionActor {
    rx: mpsc::UnboundedReceiver<SessionMsg>,
    state: SessionActorState,
}

impl SessionActor {
    /// Main loop.
    async fn run(&mut self) {
        while let Some(msg) = self.rx.recv().await {
            self.handle(msg).await;
        }
    }

    /// Handle one message.
    #[instrument(name = "session_actor", skip_all, fields(msg = ?msg))]
    async fn handle(&mut self, msg: SessionMsg) {
        match msg {
            SessionMsg::SetTrust { path, decision } => self.state.handle_set_trust(path, decision).await,
            SessionMsg::AppendHistory { entry } => self.state.handle_append_history(entry).await,
            SessionMsg::Load { name } => self.state.handle_load(name).await,
            SessionMsg::Save { name, session } => self.state.handle_save(name, session).await,
            SessionMsg::Delete { name } => self.state.handle_delete(name).await,
            SessionMsg::List => self.state.handle_list().await,
            SessionMsg::ResumeMostRecent => self.state.handle_resume_most_recent().await,
            SessionMsg::AddUserMessage { content, images } => self.state.handle_add_user_message(content, images),
            SessionMsg::AddSystemMessage { content } => self.state.handle_add_system_message(content),
            SessionMsg::AddToolMessage { id, name, content } => self.state.handle_add_tool_message(id, name, content),
            SessionMsg::UpdateToolMessage { id_contains, content } => {
                self.state.handle_update_tool_message(&id_contains, &content)
            }
            SessionMsg::AddTurnComplete { id, content } => self.state.handle_add_turn_complete(id, content),
            SessionMsg::AddErrorMessage { id, content } => self.state.handle_add_error_message(id, content),
            SessionMsg::Reset => self.state.handle_reset(),
            SessionMsg::ForkAt { index } => self.state.handle_fork_at(index),
            SessionMsg::CloneBranch => self.state.handle_clone_branch(),
            SessionMsg::PushPendingEdit { edit } => self.state.handle_push_pending_edit(edit),
            SessionMsg::DrainPendingEdits => self.state.handle_drain_pending_edits(),
            SessionMsg::ClearPendingEdits => self.state.handle_clear_pending_edits(),
            SessionMsg::Import { path } => self.state.handle_import(path).await,
            SessionMsg::Export { path, session } => self.state.handle_export(path, session).await,
        }
    }
}

// ── Spawn ────────────────────────────────────────────────────────────────────

/// Spawn a SessionActor and return (handle, stop_cell, join_handle).
pub fn spawn_session_actor(
    bus: EventBus<Event>,
) -> (SessionHandle, crate::actors::StopCell, tokio::task::JoinHandle<()>) {
    let (tx, rx) = mpsc::unbounded_channel();

    let join = tokio::spawn(async move {
        // Load trust and history on startup (inside async context)
        let trust = tokio::task::spawn_blocking(TrustManager::load)
            .await
            .unwrap_or_default();
        let store = SessionStore::default_store()
            .unwrap_or_else(|| SessionStore::new(std::env::temp_dir().join("runie_sessions")));

        let state = SessionActorState {
            bus: bus.clone(),
            trust: trust.clone(),
            store,
            session_state: SessionState::default(),
            next_id: 0,
        };

        let mut actor = SessionActor { rx, state };

        // Emit initial events
        actor.state.emit(Event::TrustLoaded { decisions: trust.decisions() });
        let entries = tokio::task::spawn_blocking(crate::input_history::load_history)
            .await
            .ok()
            .and_then(|r| r.ok())
            .unwrap_or_default();
        actor.state.emit(Event::HistoryLoaded { entries });

        actor.run().await;
    });

    (SessionHandle::new(tx), crate::actors::StopCell, join)
}


