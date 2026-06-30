//! Ractor-based SessionActor.

use std::path::PathBuf;

use ractor::async_trait;
use ractor::{Actor, ActorProcessingErr, ActorRef};

use crate::actors::ractor_adapter::spawn_ractor;
use crate::bus::EventBus;
use crate::edit_preview::EditPreview;
use crate::message::{now, Part};
use crate::model::{ChatMessage, Role, SessionState};
use crate::session::index::{SessionIndex, SessionMetadata};
use crate::session::replay::session_to_durable_events;
use crate::session::store::SessionStore;
use crate::session::tree::SessionTree;
use crate::session::Session;
use crate::trust::{TrustDecision, TrustManager};
use crate::Event;

use super::messages::SessionMsg;
use super::ractor_session_handle::RactorSessionHandle;

/// Ractor State for SessionActor — holds all mutable state.
/// EventBus is Clone and publish takes &self, no Mutex needed.
pub struct SessionActorState {
    pub bus: EventBus<Event>,
    pub trust: TrustManager,
    pub store: SessionStore,
    pub session_state: SessionState,
    pub next_id: usize,
}

impl SessionActorState {
    fn emit(&self, event: Event) {
        self.bus.publish(event);
    }

    fn emit_changed(&self) {
        let state = self.session_state.clone();
        self.emit(Event::SessionChanged {
            state: Box::new(state),
        });
    }
}

/// Ractor-based SessionActor.
pub struct RactorSessionActor;

impl RactorSessionActor {
    fn bump_time(state: &mut SessionState) {
        state.session_updated_at = now();
    }

    fn handle_add_user_message(
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
            ..Default::default()
        });
        Self::bump_time(&mut state.session_state);
        state.emit_changed();
    }

    fn handle_add_system_message(state: &mut SessionActorState, content: String) {
        state.session_state.messages.push(ChatMessage {
            role: Role::System,
            timestamp: now(),
            id: "system".to_owned(),
            parts: vec![Part::Text { content }],
            ..Default::default()
        });
        Self::bump_time(&mut state.session_state);
        state.emit_changed();
    }

    fn handle_add_tool_message(
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
            ..Default::default()
        });
        Self::bump_time(&mut state.session_state);
        state.emit_changed();
    }

    fn handle_update_tool_message(state: &mut SessionActorState, id_contains: &str, content: &str) {
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
        Self::bump_time(&mut state.session_state);
        state.emit_changed();
    }

    fn handle_add_turn_complete(state: &mut SessionActorState, id: String, content: String) {
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
        Self::bump_time(&mut state.session_state);
        state.emit_changed();
    }

    fn handle_add_error_message(state: &mut SessionActorState, id: String, content: String) {
        state.session_state.messages.push(ChatMessage {
            role: Role::Assistant,
            timestamp: now(),
            id: format!("error.{}", id),
            parts: vec![Part::Text { content }],
            ..Default::default()
        });
        Self::bump_time(&mut state.session_state);
        state.emit_changed();
    }

    fn handle_reset(state: &mut SessionActorState) {
        state.session_state = SessionState::default();
        state.emit_changed();
    }

    fn handle_fork_at(state: &mut SessionActorState, index: usize) {
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
        Self::bump_time(&mut state.session_state);
        state.emit_changed();
    }

    fn handle_clone_branch(state: &mut SessionActorState) {
        let tree = state
            .session_state
            .session_tree
            .clone()
            .unwrap_or_else(|| SessionTree::from_messages(&state.session_state.messages));
        state.session_state.session_tree = Some(tree);
        Self::bump_time(&mut state.session_state);
        state.emit_changed();
    }

    fn handle_push_pending_edit(state: &mut SessionActorState, edit: EditPreview) {
        state.session_state.pending_edits.push(edit);
        state.emit_changed();
    }

    fn handle_drain_pending_edits(state: &mut SessionActorState) {
        state.session_state.pending_edits.clear();
        state.emit_changed();
    }

    fn handle_clear_pending_edits(state: &mut SessionActorState) {
        state.session_state.pending_edits.clear();
        state.emit_changed();
    }

    async fn handle_set_trust(
        state: &mut SessionActorState,
        path: PathBuf,
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

    async fn handle_append_history(_state: &mut SessionActorState, entry: String) {
        let entry_clone = entry;
        let _ =
            tokio::task::spawn_blocking(move || crate::input_history::append_history(&entry_clone))
                .await;
    }

    async fn handle_load(state: &mut SessionActorState, name: String) {
        let store = state.store.clone();
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

    async fn handle_save(state: &mut SessionActorState, name: String, session: Session) {
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

    async fn handle_delete(state: &mut SessionActorState, name: String) {
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

    async fn handle_list(state: &mut SessionActorState) {
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

    async fn handle_import(state: &mut SessionActorState, path: PathBuf) {
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

    async fn handle_export(state: &mut SessionActorState, path: PathBuf, session: Session) {
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
}

#[async_trait]
impl Actor for RactorSessionActor {
    type Msg = SessionMsg;
    type State = SessionActorState;
    type Arguments = EventBus<Event>;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        bus: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        // Load trust and history on startup
        let trust = tokio::task::spawn_blocking(TrustManager::load)
            .await
            .unwrap_or_default();
        let store = SessionStore::default_store()
            .unwrap_or_else(|| SessionStore::new(std::env::temp_dir().join("runie_sessions")));

        let state = SessionActorState {
            bus,
            trust: trust.clone(),
            store,
            session_state: SessionState::default(),
            next_id: 0,
        };
        state.emit(Event::TrustLoaded {
            decisions: trust.decisions(),
        });
        let entries = tokio::task::spawn_blocking(crate::input_history::load_history)
            .await
            .ok()
            .and_then(|r| r.ok())
            .unwrap_or_default();
        state.emit(Event::HistoryLoaded { entries });
        Ok(state)
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        msg: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match msg {
            SessionMsg::SetTrust { path, decision } => {
                Self::handle_set_trust(state, path, decision).await
            }
            SessionMsg::AppendHistory { entry } => Self::handle_append_history(state, entry).await,
            SessionMsg::Load { name } => Self::handle_load(state, name).await,
            SessionMsg::Save { name, session } => Self::handle_save(state, name, session).await,
            SessionMsg::Delete { name } => Self::handle_delete(state, name).await,
            SessionMsg::List => Self::handle_list(state).await,
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
        Ok(())
    }
}

impl RactorSessionActor {
    /// Spawn a `RactorSessionActor` on the given event bus.
    pub async fn spawn(
        bus: EventBus<Event>,
    ) -> Result<(RactorSessionHandle, ractor::ActorCell), ractor::SpawnErr> {
        let (handle, _join, cell) = spawn_ractor(None, Self, bus).await?;
        Ok((RactorSessionHandle::new(handle), cell))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::Receiver;

    /// Wait for an event matching a predicate with a deterministic timeout.
    async fn wait_for_event<F>(sub: &mut Receiver<Event>, pred: F) -> bool
    where
        F: Fn(&Event) -> bool,
    {
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
        while tokio::time::Instant::now() < deadline {
            let timeout_duration = deadline - tokio::time::Instant::now();
            match tokio::time::timeout(timeout_duration, sub.recv()).await {
                Ok(Ok(evt)) => {
                    if pred(&evt) {
                        return true;
                    }
                }
                Ok(Err(_)) | Err(_) => break,
            }
        }
        false
    }

    #[tokio::test]
    async fn ractor_session_actor_spawns() {
        let bus = EventBus::<Event>::new(16);
        let result = RactorSessionActor::spawn(bus).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn ractor_session_handles_trust_loaded() {
        let bus = EventBus::<Event>::new(16);
        let mut sub = bus.subscribe();
        let (_handle, _cell) = RactorSessionActor::spawn(bus).await.unwrap();

        let found = wait_for_event(&mut sub, |e| matches!(e, Event::TrustLoaded { .. })).await;
        assert!(found, "Expected TrustLoaded event");
    }

    #[tokio::test]
    async fn ractor_session_adds_user_message() {
        let bus = EventBus::<Event>::new(16);
        let mut sub = bus.subscribe();
        let (handle, _cell) = RactorSessionActor::spawn(bus).await.unwrap();

        handle.try_add_user_message("hello".to_string(), vec![]);

        let found = wait_for_event(&mut sub, |e| matches!(e, Event::SessionChanged { .. })).await;
        assert!(
            found,
            "Expected SessionChanged event after adding user message"
        );
    }
}
