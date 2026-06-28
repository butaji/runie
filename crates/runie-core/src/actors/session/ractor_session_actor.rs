//! Ractor-based SessionActor.

use std::path::PathBuf;
use std::sync::Mutex;

use ractor::{Actor, ActorProcessingErr, ActorRef};
use ractor::async_trait;

use crate::actors::ractor_adapter::{spawn_ractor, EventBusBridge};
use crate::bus::EventBus;
use crate::edit_preview::EditPreview;
use crate::message::{now, Part};
use crate::model::{ChatMessage, Role, SessionState};
use crate::session::index::{SessionIndex, SessionMetadata};
use crate::session::replay::session_to_durable_events;
use crate::session::store::SessionStore;
use crate::session::Session;
use crate::session::tree::SessionTree;
use crate::trust::{TrustDecision, TrustManager};
use crate::Event;

use super::messages::SessionMsg;
use super::ractor_session_handle::RactorSessionHandle;

/// Ractor-based SessionActor.
pub struct RactorSessionActor {
    bus_bridge: EventBusBridge<Event>,
    trust: Mutex<TrustManager>,
    store: SessionStore,
    session_state: Mutex<SessionState>,
    next_id: Mutex<usize>,
}

impl Default for RactorSessionActor {
    fn default() -> Self {
        let store = SessionStore::default_store()
            .unwrap_or_else(|| SessionStore::new(std::env::temp_dir().join("runie_sessions")));
        Self {
            bus_bridge: EventBusBridge::new(EventBus::new(16)),
            trust: Mutex::new(TrustManager::default()),
            store,
            session_state: Mutex::new(SessionState::default()),
            next_id: Mutex::new(0),
        }
    }
}

impl RactorSessionActor {
    fn new(bus: EventBus<Event>) -> Self {
        let store = SessionStore::default_store()
            .unwrap_or_else(|| SessionStore::new(std::env::temp_dir().join("runie_sessions")));
        Self {
            bus_bridge: EventBusBridge::new(bus),
            trust: Mutex::new(TrustManager::default()),
            store,
            session_state: Mutex::new(SessionState::default()),
            next_id: Mutex::new(0),
        }
    }

    /// Spawn a `RactorSessionActor` on the given event bus.
    pub async fn spawn(bus: EventBus<Event>) -> Result<(RactorSessionHandle, ractor::ActorCell), ractor::SpawnErr> {
        let actor = Self::new(bus.clone());
        let (handle, _join, cell) = spawn_ractor(None, actor, bus).await?;
        Ok((RactorSessionHandle::new(handle), cell))
    }

    fn emit(&self, event: Event) {
        self.bus_bridge.publish(event);
    }

    fn emit_changed(&self) {
        let state = self.session_state.lock().unwrap().clone();
        self.emit(Event::SessionChanged { state: Box::new(state) });
    }

    fn bump_time(state: &mut SessionState) {
        state.session_updated_at = now();
    }
}

impl RactorSessionActor {
    fn handle_add_user_message(&self, content: String, images: Vec<String>) {
        let mut state = self.session_state.lock().unwrap();
        let id = {
            let mut next_id = self.next_id.lock().unwrap();
            let id = format!("req.{}", *next_id);
            *next_id += 1;
            id
        };
        state.image_attachments.extend(images);
        state.messages.push(ChatMessage {
            role: Role::User,
            timestamp: now(),
            id,
            parts: vec![Part::Text { content }],
            ..Default::default()
        });
        Self::bump_time(&mut state);
        drop(state);
        self.emit_changed();
    }

    fn handle_add_system_message(&self, content: String) {
        let mut state = self.session_state.lock().unwrap();
        state.messages.push(ChatMessage {
            role: Role::System,
            timestamp: now(),
            id: "system".to_owned(),
            parts: vec![Part::Text { content }],
            ..Default::default()
        });
        Self::bump_time(&mut state);
        drop(state);
        self.emit_changed();
    }

    fn handle_add_tool_message(&self, id: String, name: String, content: String) {
        let mut state = self.session_state.lock().unwrap();
        state.messages.push(ChatMessage {
            role: Role::Tool,
            timestamp: now(),
            id,
            parts: vec![Part::Text { content }],
            tool_call_id: Some(name),
            ..Default::default()
        });
        Self::bump_time(&mut state);
        drop(state);
        self.emit_changed();
    }

    fn handle_update_tool_message(&self, id_contains: &str, content: &str) {
        let mut state = self.session_state.lock().unwrap();
        if let Some(idx) = state.messages.iter().rposition(|m| m.role == Role::Tool && m.id.contains(id_contains)) {
            if let Some(msg) = state.messages.get_mut(idx) {
                msg.set_text_part(content.to_owned());
                msg.timestamp = now();
            }
        }
        Self::bump_time(&mut state);
        drop(state);
        self.emit_changed();
    }

    fn handle_add_turn_complete(&self, id: String, content: String) {
        let mut state = self.session_state.lock().unwrap();
        if let Some(idx) = state.messages.iter().position(|m| m.role == Role::TurnComplete && m.id == id) {
            if let Some(msg) = state.messages.get_mut(idx) {
                msg.set_text_part(content);
                msg.timestamp = now();
            }
        } else {
            state.messages.push(ChatMessage {
                role: Role::TurnComplete,
                timestamp: now(),
                id,
                parts: vec![Part::Text { content }],
                ..Default::default()
            });
        }
        Self::bump_time(&mut state);
        drop(state);
        self.emit_changed();
    }

    fn handle_add_error_message(&self, id: String, content: String) {
        let mut state = self.session_state.lock().unwrap();
        state.messages.push(ChatMessage {
            role: Role::Assistant,
            timestamp: now(),
            id: format!("error.{}", id),
            parts: vec![Part::Text { content }],
            ..Default::default()
        });
        Self::bump_time(&mut state);
        drop(state);
        self.emit_changed();
    }

    fn handle_reset(&self) {
        let mut state = self.session_state.lock().unwrap();
        *state = SessionState::default();
        drop(state);
        self.emit_changed();
    }

    fn handle_fork_at(&self, index: usize) {
        let mut state = self.session_state.lock().unwrap();
        match state.session_tree.as_mut() {
            Some(tree) => {
                if let Some(path) = tree.fork_at(index) {
                    tree.navigate_to(&path);
                }
            }
            None => {
                let tree = SessionTree::from_messages(&state.messages);
                let mut new_tree = tree;
                if let Some(path) = new_tree.fork_at(index) {
                    new_tree.navigate_to(&path);
                }
                state.session_tree = Some(new_tree);
            }
        }
        Self::bump_time(&mut state);
        drop(state);
        self.emit_changed();
    }

    fn handle_clone_branch(&self) {
        let mut state = self.session_state.lock().unwrap();
        let tree = state.session_tree.clone()
            .unwrap_or_else(|| SessionTree::from_messages(&state.messages));
        state.session_tree = Some(tree);
        Self::bump_time(&mut state);
        drop(state);
        self.emit_changed();
    }

    fn handle_push_pending_edit(&self, edit: EditPreview) {
        let mut state = self.session_state.lock().unwrap();
        state.pending_edits.push(edit);
        drop(state);
        self.emit_changed();
    }

    fn handle_drain_pending_edits(&self) {
        let mut state = self.session_state.lock().unwrap();
        state.pending_edits.clear();
        drop(state);
        self.emit_changed();
    }

    fn handle_clear_pending_edits(&self) {
        let mut state = self.session_state.lock().unwrap();
        state.pending_edits.clear();
        drop(state);
        self.emit_changed();
    }

    async fn handle_set_trust(&self, path: PathBuf, decision: TrustDecision) {
        {
            let mut trust = self.trust.lock().unwrap();
            trust.set(&path, decision);
        }
        let trust = self.trust.lock().unwrap().clone();
        let path_clone = path.clone();
        let decision_clone = decision;
        let _ = tokio::task::spawn_blocking(move || trust.save()).await;
        self.emit(Event::TrustChanged { path: path_clone, decision: decision_clone });
    }

    async fn handle_append_history(&self, entry: String) {
        let entry_clone = entry;
        let _ = tokio::task::spawn_blocking(move || crate::input_history::append_history(&entry_clone)).await;
    }

    async fn handle_load(&self, name: String) {
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
        }).await;

        match res {
            Ok(Ok((events, metadata))) => {
                self.emit(Event::SessionLoaded {
                    name,
                    events: Box::new(events),
                    metadata: metadata.map(Box::new),
                });
            }
            _ => {
                self.emit(Event::SessionOperationFailed {
                    operation: "load".to_owned(),
                    error: format!("Session '{}' not found. Use /sessions to list saved sessions.", name),
                });
            }
        }
    }

    async fn handle_save(&self, name: String, session: Session) {
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
        };

        let res = tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            store.append_batch(&name_for_task, &events)?;
            store.update_index(&meta)?;
            Ok(())
        }).await;

        match res {
            Ok(Ok(())) => self.emit(Event::SessionSaved { name }),
            Ok(Err(e)) => self.emit(Event::SessionOperationFailed { operation: "save".to_owned(), error: e.to_string() }),
            Err(e) => self.emit(Event::SessionOperationFailed { operation: "save".to_owned(), error: e.to_string() }),
        }
    }

    async fn handle_delete(&self, name: String) {
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
            Err(e) => self.emit(Event::SessionOperationFailed { operation: "delete".to_owned(), error: e.to_string() }),
        }
    }

    async fn handle_list(&self) {
        let store = self.store.clone();
        let res = tokio::task::spawn_blocking(move || store.list()).await;

        match res {
            Ok(Ok(sessions)) => self.emit(Event::SessionList { sessions: Box::new(sessions) }),
            Ok(Err(e)) => self.emit(Event::SessionOperationFailed { operation: "list".to_owned(), error: e.to_string() }),
            Err(e) => self.emit(Event::SessionOperationFailed { operation: "list".to_owned(), error: e.to_string() }),
        }
    }

    async fn handle_import(&self, path: PathBuf) {
        let path_clone = path.clone();
        match tokio::task::spawn_blocking(move || std::fs::read_to_string(&path_clone)).await {
            Ok(Ok(json)) => {
                match serde_json::from_str::<Session>(&json) {
                    Ok(session) => self.emit(Event::SessionImported { session: Box::new(session) }),
                    Err(e) => self.emit(Event::SessionOperationFailed { operation: "import".to_owned(), error: e.to_string() }),
                }
            }
            Ok(Err(e)) => self.emit(Event::SessionOperationFailed { operation: "import".to_owned(), error: e.to_string() }),
            Err(e) => self.emit(Event::SessionOperationFailed { operation: "import".to_owned(), error: e.to_string() }),
        }
    }

    async fn handle_export(&self, path: PathBuf, session: Session) {
        let path_clone = path.clone();
        let json = match serde_json::to_string_pretty(&session) {
            Ok(json) => json,
            Err(e) => {
                self.emit(Event::SessionOperationFailed { operation: "export".to_owned(), error: e.to_string() });
                return;
            }
        };
        match tokio::task::spawn_blocking(move || std::fs::write(&path, json)).await {
            Ok(Ok(())) => self.emit(Event::SessionExported { path: path_clone.to_string_lossy().to_string() }),
            Ok(Err(e)) => self.emit(Event::SessionOperationFailed { operation: "export".to_owned(), error: e.to_string() }),
            Err(e) => self.emit(Event::SessionOperationFailed { operation: "export".to_owned(), error: e.to_string() }),
        }
    }
}

#[async_trait]
impl Actor for RactorSessionActor {
    type Msg = SessionMsg;
    type State = ();
    type Arguments = EventBus<Event>;

    async fn pre_start(&self, _myself: ActorRef<Self::Msg>, _args: Self::Arguments) -> Result<Self::State, ActorProcessingErr> {
        // Load trust and history on startup
        let trust = tokio::task::spawn_blocking(TrustManager::load).await.unwrap_or_default();
        self.emit(Event::TrustLoaded { decisions: trust.decisions() });
        let entries = tokio::task::spawn_blocking(crate::input_history::load_history)
            .await.ok().and_then(|r| r.ok()).unwrap_or_default();
        self.emit(Event::HistoryLoaded { entries });
        Ok(())
    }

    async fn handle(&self, _myself: ActorRef<Self::Msg>, msg: Self::Msg, _state: &mut Self::State) -> Result<(), ActorProcessingErr> {
        match msg {
            SessionMsg::SetTrust { path, decision } => self.handle_set_trust(path, decision).await,
            SessionMsg::AppendHistory { entry } => self.handle_append_history(entry).await,
            SessionMsg::Load { name } => self.handle_load(name).await,
            SessionMsg::Save { name, session } => self.handle_save(name, session).await,
            SessionMsg::Delete { name } => self.handle_delete(name).await,
            SessionMsg::List => self.handle_list().await,
            SessionMsg::AddUserMessage { content, images } => self.handle_add_user_message(content, images),
            SessionMsg::AddSystemMessage { content } => self.handle_add_system_message(content),
            SessionMsg::AddToolMessage { id, name, content } => self.handle_add_tool_message(id, name, content),
            SessionMsg::UpdateToolMessage { id_contains, content } => self.handle_update_tool_message(&id_contains, &content),
            SessionMsg::AddTurnComplete { id, content } => self.handle_add_turn_complete(id, content),
            SessionMsg::AddErrorMessage { id, content } => self.handle_add_error_message(id, content),
            SessionMsg::Reset => self.handle_reset(),
            SessionMsg::ForkAt { index } => self.handle_fork_at(index),
            SessionMsg::CloneBranch => self.handle_clone_branch(),
            SessionMsg::PushPendingEdit { edit } => self.handle_push_pending_edit(edit),
            SessionMsg::DrainPendingEdits => self.handle_drain_pending_edits(),
            SessionMsg::ClearPendingEdits => self.handle_clear_pending_edits(),
            SessionMsg::Import { path } => self.handle_import(path).await,
            SessionMsg::Export { path, session } => self.handle_export(path, session).await,
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let (handle, _cell) = RactorSessionActor::spawn(bus).await.unwrap();

        // Wait for TrustLoaded event
        let mut found = false;
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(200);
        while tokio::time::Instant::now() < deadline {
            if let Ok(e) = sub.try_recv() {
                if matches!(e, Event::TrustLoaded { .. }) {
                    found = true;
                    break;
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
        assert!(found, "Expected TrustLoaded event");
    }

    #[tokio::test]
    async fn ractor_session_adds_user_message() {
        let bus = EventBus::<Event>::new(16);
        let mut sub = bus.subscribe();
        let (handle, _cell) = RactorSessionActor::spawn(bus).await.unwrap();

        handle.try_add_user_message("hello".to_string(), vec![]);

        // Wait for SessionChanged event
        let mut found = false;
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(200);
        while tokio::time::Instant::now() < deadline {
            if let Ok(e) = sub.try_recv() {
                if matches!(e, Event::SessionChanged { .. }) {
                    found = true;
                    break;
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
        assert!(found, "Expected SessionChanged event after adding user message");
    }
}
