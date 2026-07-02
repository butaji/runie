//! Async message handlers for `RactorSessionActor`.
//!
//! Extracted from `ractor_session_actor.rs` to satisfy the 500-line file limit.

use camino::Utf8PathBuf;
use std::path::PathBuf;
use tracing::instrument;

use crate::bus::EventBus;
use crate::edit_preview::EditPreview;
use crate::message::{now, Part, MessageOrigin};
use crate::proto::message::MessageMetadata;
use crate::model::{ChatMessage, Role};
use crate::session::SessionMetadata;
use crate::session::replay::session_to_durable_events;
use crate::session::store::SessionStore;
use crate::session::tree::SessionTree;
use crate::session::Session;
use crate::trust::{TrustDecision, TrustManager};
use crate::Event;

use super::messages::SessionMsg;

/// Ractor-based SessionActor.
pub struct RactorSessionActor;

/// Ractor State for SessionActor — holds all mutable state.
/// EventBus is Clone and publish takes &self, no Mutex needed.
pub struct SessionActorState {
    pub bus: EventBus<Event>,
    pub trust: TrustManager,
    pub store: SessionStore,
    pub session_state: crate::model::SessionState,
    pub next_id: usize,
}

impl SessionActorState {
    pub fn emit(&self, event: Event) {
        self.bus.publish(event);
    }

    pub fn emit_changed(&self) {
        let state = self.session_state.clone();
        self.emit(Event::SessionChanged {
            state: Box::new(state),
        });
    }
}

/// Bump the session updated timestamp.
fn bump_time(state: &mut crate::model::SessionState) {
    state.session_updated_at = now();
}

impl RactorSessionActor {
    /// Handle add user message.
    pub fn handle_add_user_message(
        state: &mut SessionActorState,
        content: String,
        images: Vec<String>,
    ) {
        let id = {
            let id = format!("req.{}", state.next_id);
            state.next_id += 1;
            id
        };
        state.session_state.image_attachments.extend(images);
        state.session_state.messages.push(ChatMessage {
            role: Role::User,
            timestamp: now(),
            id,
            parts: vec![Part::Text { content }],
            metadata: MessageMetadata {
                origin: MessageOrigin::User,
                ..Default::default()
            },
            ..Default::default()
        });
        bump_time(&mut state.session_state);
        state.emit_changed();
    }

    /// Handle add system message.
    pub fn handle_add_system_message(state: &mut SessionActorState, content: String) {
        state.session_state.messages.push(ChatMessage {
            role: Role::System,
            timestamp: now(),
            id: "system".to_owned(),
            parts: vec![Part::Text { content }],
            metadata: MessageMetadata {
                origin: MessageOrigin::System,
                ..Default::default()
            },
            ..Default::default()
        });
        bump_time(&mut state.session_state);
        state.emit_changed();
    }

    /// Handle add tool message.
    pub fn handle_add_tool_message(
        state: &mut SessionActorState,
        id: String,
        name: String,
        content: String,
    ) {
        state.session_state.messages.push(ChatMessage {
            role: Role::Tool,
            timestamp: now(),
            id,
            parts: vec![Part::Text { content }],
            tool_call_id: Some(name),
            metadata: MessageMetadata {
                origin: MessageOrigin::Tool,
                ..Default::default()
            },
            ..Default::default()
        });
        bump_time(&mut state.session_state);
        state.emit_changed();
    }

    /// Handle update tool message.
    pub fn handle_update_tool_message(state: &mut SessionActorState, id_contains: &str, content: &str) {
        if let Some(idx) = state
            .session_state
            .messages
            .iter()
            .rposition(|m| m.role == Role::Tool && m.id.contains(id_contains))
        {
            if let Some(msg) = state.session_state.messages.get_mut(idx) {
                msg.set_text_part(content.to_owned());
                msg.timestamp = now();
            }
        }
        bump_time(&mut state.session_state);
        state.emit_changed();
    }

    /// Handle add turn complete message.
    pub fn handle_add_turn_complete(state: &mut SessionActorState, id: String, content: String) {
        if let Some(idx) = state
            .session_state
            .messages
            .iter()
            .position(|m| m.role == Role::TurnComplete && m.id == id)
        {
            if let Some(msg) = state.session_state.messages.get_mut(idx) {
                msg.set_text_part(content);
                msg.timestamp = now();
            }
        } else {
            state.session_state.messages.push(ChatMessage {
                role: Role::TurnComplete,
                timestamp: now(),
                id,
                parts: vec![Part::Text { content }],
                ..Default::default()
            });
        }
        bump_time(&mut state.session_state);
        state.emit_changed();
    }

    /// Handle add error message.
    pub fn handle_add_error_message(state: &mut SessionActorState, id: String, content: String) {
        state.session_state.messages.push(ChatMessage {
            role: Role::Assistant,
            timestamp: now(),
            id: format!("error.{}", id),
            parts: vec![Part::Text { content }],
            ..Default::default()
        });
        bump_time(&mut state.session_state);
        state.emit_changed();
    }

    /// Handle reset session state.
    pub fn handle_reset(state: &mut SessionActorState) {
        state.session_state = crate::model::SessionState::default();
        state.emit_changed();
    }

    /// Handle fork session at index.
    pub fn handle_fork_at(state: &mut SessionActorState, index: usize) {
        match state.session_state.session_tree.as_mut() {
            Some(tree) => {
                if let Some(path) = tree.fork_at(index) {
                    tree.navigate_to(&path);
                }
            }
            None => {
                let tree = SessionTree::from_messages(&state.session_state.messages);
                let mut new_tree = tree;
                if let Some(path) = new_tree.fork_at(index) {
                    new_tree.navigate_to(&path);
                }
                state.session_state.session_tree = Some(new_tree);
            }
        }
        bump_time(&mut state.session_state);
        state.emit_changed();
    }

    /// Handle clone branch.
    pub fn handle_clone_branch(state: &mut SessionActorState) {
        let tree = state
            .session_state
            .session_tree
            .clone()
            .unwrap_or_else(|| SessionTree::from_messages(&state.session_state.messages));
        state.session_state.session_tree = Some(tree);
        bump_time(&mut state.session_state);
        state.emit_changed();
    }

    /// Handle push pending edit.
    pub fn handle_push_pending_edit(state: &mut SessionActorState, edit: EditPreview) {
        state.session_state.pending_edits.push(edit);
        state.emit_changed();
    }

    /// Handle drain pending edits.
    pub fn handle_drain_pending_edits(state: &mut SessionActorState) {
        state.session_state.pending_edits.clear();
        state.emit_changed();
    }

    /// Handle clear pending edits.
    pub fn handle_clear_pending_edits(state: &mut SessionActorState) {
        state.session_state.pending_edits.clear();
        state.emit_changed();
    }

    /// Handle set trust decision.
    pub async fn handle_set_trust(
        state: &mut SessionActorState,
        path: Utf8PathBuf,
        decision: TrustDecision,
    ) {
        state.trust.set(&path, decision);
        let trust = state.trust.clone();
        let path_clone = path.clone();
        let decision_clone = decision;
        let _ = tokio::task::spawn_blocking(move || trust.save()).await;
        state.emit(Event::TrustChanged {
            path: path_clone,
            decision: decision_clone,
        });
    }

    /// Handle append to history.
    pub async fn handle_append_history(_state: &mut SessionActorState, entry: String) {
        let entry_clone = entry;
        let _ =
            tokio::task::spawn_blocking(move || crate::input_history::append_history(&entry_clone))
                .await;
    }

    /// Handle load session.
    pub async fn handle_load(state: &mut SessionActorState, name: String) {
        let store = state.store.clone();
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
                state.emit(Event::SessionLoaded {
                    name,
                    events: Box::new(events),
                    metadata: metadata.map(Box::new),
                });
            }
            _ => {
                state.emit(Event::SessionOperationFailed {
                    operation: "load".to_owned(),
                    error: format!(
                        "Session '{}' not found. Use /sessions to list saved sessions.",
                        name
                    ),
                });
            }
        }
    }

    /// Handle save session.
    pub async fn handle_save(state: &mut SessionActorState, name: String, session: Session) {
        let store = state.store.clone();
        let name_for_task = name.clone();
        let events = session_to_durable_events(&session);
        let meta = SessionMetadata {
            id: name_for_task.clone(),
            display_name: session
                .display_name
                .clone()
                .unwrap_or_else(|| name_for_task.clone()),
            created_at: session.created_at,
            updated_at: now(),
            message_count: session.messages.len(),
            summary: None,
            is_starred: false,
            is_system: false,
        };

        let res = tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            store.append_batch(&name_for_task, &events)?;
            store.update_metadata(&meta)?;
            Ok(())
        })
        .await;

        match res {
            Ok(Ok(())) => state.emit(Event::SessionSaved { name }),
            Ok(Err(e)) => state.emit(Event::SessionOperationFailed {
                operation: "save".to_owned(),
                error: e.to_string(),
            }),
            Err(e) => state.emit(Event::SessionOperationFailed {
                operation: "save".to_owned(),
                error: e.to_string(),
            }),
        }
    }

    /// Handle delete session.
    pub async fn handle_delete(state: &mut SessionActorState, name: String) {
        let store = state.store.clone();
        let name_for_task = name.clone();
        let res = tokio::task::spawn_blocking(move || store.delete(&name_for_task)).await;

        match res {
            Ok(Ok(())) => state.emit(Event::SessionDeleted { name }),
            Ok(Err(_)) => {
                state.emit(Event::SessionOperationFailed {
                    operation: "delete".to_owned(),
                    error: format!(
                        "Session '{}' not found. Use /sessions to list saved sessions.",
                        name
                    ),
                });
            }
            Err(e) => state.emit(Event::SessionOperationFailed {
                operation: "delete".to_owned(),
                error: e.to_string(),
            }),
        }
    }

    /// Handle list sessions.
    pub async fn handle_list(state: &mut SessionActorState) {
        let store = state.store.clone();
        let res = tokio::task::spawn_blocking(move || store.list()).await;

        match res {
            Ok(Ok(sessions)) => state.emit(Event::SessionList {
                sessions: Box::new(sessions),
            }),
            Ok(Err(e)) => state.emit(Event::SessionOperationFailed {
                operation: "list".to_owned(),
                error: e.to_string(),
            }),
            Err(e) => state.emit(Event::SessionOperationFailed {
                operation: "list".to_owned(),
                error: e.to_string(),
            }),
        }
    }

    /// Handle resume most recent session.
    pub async fn handle_resume_most_recent(state: &mut SessionActorState) {
        let store = state.store.clone();
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
        }).await;

        match res {
            Ok(Some(name)) => {
                Self::handle_load(state, name).await;
            }
            Ok(None) => {
                state.emit(Event::SessionOperationFailed {
                    operation: "resume".to_owned(),
                    error: "No sessions to resume.".to_owned(),
                });
            }
            Err(e) => {
                state.emit(Event::SessionOperationFailed {
                    operation: "resume".to_owned(),
                    error: e.to_string(),
                });
            }
        }
    }

    /// Handle import session.
    pub async fn handle_import(state: &mut SessionActorState, path: PathBuf) {
        let path_clone = path.clone();
        match tokio::task::spawn_blocking(move || std::fs::read_to_string(&path_clone)).await {
            Ok(Ok(json)) => match serde_json::from_str::<Session>(&json) {
                Ok(session) => state.emit(Event::SessionImported {
                    session: Box::new(session),
                }),
                Err(e) => state.emit(Event::SessionOperationFailed {
                    operation: "import".to_owned(),
                    error: e.to_string(),
                }),
            },
            Ok(Err(e)) => state.emit(Event::SessionOperationFailed {
                operation: "import".to_owned(),
                error: e.to_string(),
            }),
            Err(e) => state.emit(Event::SessionOperationFailed {
                operation: "import".to_owned(),
                error: e.to_string(),
            }),
        }
    }

    /// Handle export session.
    pub async fn handle_export(state: &mut SessionActorState, path: PathBuf, session: Session) {
        let path_clone = path.clone();
        let json = match serde_json::to_string_pretty(&session) {
            Ok(json) => json,
            Err(e) => {
                state.emit(Event::SessionOperationFailed {
                    operation: "export".to_owned(),
                    error: e.to_string(),
                });
                return;
            }
        };
        match tokio::task::spawn_blocking(move || std::fs::write(&path, json)).await {
            Ok(Ok(())) => state.emit(Event::SessionExported {
                path: path_clone.to_string_lossy().to_string(),
            }),
            Ok(Err(e)) => state.emit(Event::SessionOperationFailed {
                operation: "export".to_owned(),
                error: e.to_string(),
            }),
            Err(e) => state.emit(Event::SessionOperationFailed {
                operation: "export".to_owned(),
                error: e.to_string(),
            }),
        }
    }

    /// Dispatch incoming session messages to their handlers.
    #[instrument(name = "session_handler", skip_all, fields(msg = ?msg))]
    pub async fn handle_msg(state: &mut SessionActorState, msg: SessionMsg) {
        match msg {
            SessionMsg::SetTrust { path, decision } => {
                Self::handle_set_trust(state, path, decision).await
            }
            SessionMsg::AppendHistory { entry } => Self::handle_append_history(state, entry).await,
            SessionMsg::Load { name } => Self::handle_load(state, name).await,
            SessionMsg::Save { name, session } => Self::handle_save(state, name, session).await,
            SessionMsg::Delete { name } => Self::handle_delete(state, name).await,
            SessionMsg::List => Self::handle_list(state).await,
            SessionMsg::ResumeMostRecent => Self::handle_resume_most_recent(state).await,
            SessionMsg::AddUserMessage { content, images } => {
                Self::handle_add_user_message(state, content, images)
            }
            SessionMsg::AddSystemMessage { content } => {
                Self::handle_add_system_message(state, content)
            }
            SessionMsg::AddToolMessage { id, name, content } => {
                Self::handle_add_tool_message(state, id, name, content)
            }
            SessionMsg::UpdateToolMessage {
                id_contains,
                content,
            } => Self::handle_update_tool_message(state, &id_contains, &content),
            SessionMsg::AddTurnComplete { id, content } => {
                Self::handle_add_turn_complete(state, id, content)
            }
            SessionMsg::AddErrorMessage { id, content } => {
                Self::handle_add_error_message(state, id, content)
            }
            SessionMsg::Reset => Self::handle_reset(state),
            SessionMsg::ForkAt { index } => Self::handle_fork_at(state, index),
            SessionMsg::CloneBranch => Self::handle_clone_branch(state),
            SessionMsg::PushPendingEdit { edit } => Self::handle_push_pending_edit(state, edit),
            SessionMsg::DrainPendingEdits => Self::handle_drain_pending_edits(state),
            SessionMsg::ClearPendingEdits => Self::handle_clear_pending_edits(state),
            SessionMsg::Import { path } => Self::handle_import(state, path).await,
            SessionMsg::Export { path, session } => Self::handle_export(state, path, session).await,
        }
    }
}
