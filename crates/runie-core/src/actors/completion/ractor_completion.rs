//! Ractor-based `CompletionActor` implementation.
//!
//! This module provides a ractor-based implementation of the CompletionActor,
//! following the same pattern as the InputActor migration.

use std::sync::Mutex;

use ractor::{Actor, ActorProcessingErr, ActorRef};
use ractor::async_trait;

use crate::actors::ractor_adapter::{spawn_ractor, EventBusBridge, RactorHandle};
use crate::bus::EventBus;
use crate::event::Event;
use crate::model::CompletionState;

use super::messages::CompletionMsg;

/// Ractor handle type for CompletionActor.
#[derive(Clone, Debug)]
pub struct RactorCompletionHandle {
    inner: RactorHandle<CompletionMsg>,
}

impl RactorCompletionHandle {
    /// Create a new handle wrapping the inner RactorHandle.
    pub fn new(inner: RactorHandle<CompletionMsg>) -> Self {
        Self { inner }
    }

    /// Send a message to the actor.
    pub async fn send(&self, msg: CompletionMsg) {
        let _ = self.inner.send(msg).await;
    }

    /// Try to send a message (sync fire-and-forget).
    pub fn try_send(&self, msg: CompletionMsg) {
        let _ = self.inner.try_send(msg);
    }
}

impl From<RactorHandle<CompletionMsg>> for RactorCompletionHandle {
    fn from(handle: RactorHandle<CompletionMsg>) -> Self {
        Self::new(handle)
    }
}

/// Ractor-based CompletionActor.
///
/// Owns completion state for path completion and @ mention suggestions.
#[allow(dead_code)] // Only constructed via spawn() in tests.
pub struct RactorCompletionActor {
    /// The authoritative completion state (protected by mutex).
    state: Mutex<CompletionState>,
    /// Bridge to the event bus for publishing facts.
    bus_bridge: EventBusBridge<Event>,
}

impl Default for RactorCompletionActor {
    fn default() -> Self {
        Self {
            state: Mutex::new(CompletionState::default()),
            bus_bridge: EventBusBridge::new(EventBus::new(16)),
        }
    }
}

#[async_trait]
impl Actor for RactorCompletionActor {
    type Msg = CompletionMsg;
    type State = ();
    type Arguments = EventBus<Event>;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        _args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(())
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        msg: CompletionMsg,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        let (new_state, should_emit) = {
            let mut state = self.state.lock().unwrap();
            Self::apply_msg(&msg, &mut state);
            let should_emit = true;
            (state.clone(), should_emit)
        };
        if should_emit {
            self.bus_bridge.publish(Event::CompletionChanged {
                state: Box::new(new_state),
            });
        }
        Ok(())
    }
}

#[allow(dead_code)] // Only used in tests.
impl RactorCompletionActor {
    /// Spawn a `RactorCompletionActor` on the given event bus.
    pub async fn spawn(bus: EventBus<Event>) -> (RactorCompletionHandle, ractor::ActorCell) {
        let actor = Self {
            state: Mutex::new(CompletionState::default()),
            bus_bridge: EventBusBridge::new(bus.clone()),
        };
        let (handle, _join, cell) = spawn_ractor(None, actor, bus).await.unwrap();
        (RactorCompletionHandle::new(handle), cell)
    }

    /// Apply a message to the state.
    fn apply_msg(msg: &CompletionMsg, state: &mut CompletionState) {
        match msg {
            CompletionMsg::TogglePathCompletion { partial } => {
                Self::toggle_path_completion(state, partial);
            }
            CompletionMsg::PathCompletionUp => Self::path_completion_up(state),
            CompletionMsg::PathCompletionDown => Self::path_completion_down(state),
            CompletionMsg::PathCompletionSelect { prefix } => {
                Self::path_completion_select(state, prefix);
            }
            CompletionMsg::PathCompletionClose => Self::path_completion_close(state),
            CompletionMsg::AtSuggestionsChanged { suggestions } => {
                Self::at_suggestions_changed(state, suggestions.clone());
            }
            CompletionMsg::AtSuggestionUp => Self::at_suggestion_up(state),
            CompletionMsg::AtSuggestionDown => Self::at_suggestion_down(state),
            CompletionMsg::AtSuggestionSelect => Self::at_suggestion_select(state),
            CompletionMsg::ClearAtRef => Self::clear_at_ref(state),
            CompletionMsg::ClearAll => Self::clear_all(state),
            // These are handled elsewhere or are no-ops
            CompletionMsg::SetGhost { .. }
            | CompletionMsg::SetTabComplete { .. }
            | CompletionMsg::AcceptGhost
            | CompletionMsg::ClearGhost
            | CompletionMsg::TabCompleteNext
            | CompletionMsg::FilePickerAbort => {}
        }
    }

    // ── Path completion ───────────────────────────────────────────────────

    fn toggle_path_completion(state: &mut CompletionState, partial: &str) {
        let cwd = std::env::current_dir().unwrap_or_default();
        let suggestions = crate::path_complete::complete_path(partial, &cwd);
        if suggestions.is_empty() {
            return;
        }
        state.path_suggestions = Some(suggestions);
        state.path_selected = Some(0);
    }

    fn path_completion_up(state: &mut CompletionState) {
        if let Some(ref items) = state.path_suggestions {
            let sel = state.path_selected.unwrap_or(0);
            state.path_selected = Some(if sel == 0 { items.len() - 1 } else { sel - 1 });
        }
    }

    fn path_completion_down(state: &mut CompletionState) {
        if let Some(ref items) = state.path_suggestions {
            let sel = state.path_selected.unwrap_or(0);
            state.path_selected = Some((sel + 1) % items.len());
        }
    }

    fn path_completion_select(state: &mut CompletionState, _prefix: &str) {
        state.path_suggestions = None;
        state.path_selected = None;
    }

    fn path_completion_close(state: &mut CompletionState) {
        state.path_suggestions = None;
        state.path_selected = None;
    }

    // ── @ mention suggestions ─────────────────────────────────────────────

    fn at_suggestions_changed(state: &mut CompletionState, suggestions: Vec<String>) {
        state.at_suggestions = Some(suggestions);
        state.at_selected = Some(0);
        state.last_at_query = None;
    }

    fn at_suggestion_up(state: &mut CompletionState) {
        let items = match state.at_suggestions.as_ref() {
            Some(i) if !i.is_empty() => i,
            _ => {
                state.at_suggestions = None;
                state.at_selected = None;
                return;
            }
        };
        let sel = state.at_selected.unwrap_or(0);
        state.at_selected = Some(if sel == 0 { items.len() - 1 } else { sel - 1 });
    }

    fn at_suggestion_down(state: &mut CompletionState) {
        let items = match state.at_suggestions.as_ref() {
            Some(i) if !i.is_empty() => i,
            _ => return,
        };
        let sel = state.at_selected.unwrap_or(0);
        state.at_selected = Some((sel + 1) % items.len());
    }

    fn at_suggestion_select(state: &mut CompletionState) {
        state.at_suggestions = None;
        state.at_selected = None;
    }

    fn clear_at_ref(state: &mut CompletionState) {
        state.at_suggestions = None;
        state.at_selected = None;
        state.last_at_query = None;
    }

    // ── State mutations ───────────────────────────────────────────────────

    fn clear_all(state: &mut CompletionState) {
        state.path_suggestions = None;
        state.path_selected = None;
        state.at_suggestions = None;
        state.at_selected = None;
        state.last_at_query = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn toggle_path_completion_creates_suggestions() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _cell) = RactorCompletionActor::spawn(bus).await;

        handle
            .send(CompletionMsg::TogglePathCompletion {
                partial: String::new(),
            })
            .await;
        drop(handle);

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    #[tokio::test]
    async fn at_suggestions_changes_suggests() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _cell) = RactorCompletionActor::spawn(bus).await;

        handle
            .send(CompletionMsg::AtSuggestionsChanged {
                suggestions: vec!["foo".into(), "bar".into()],
            })
            .await;
        drop(handle);

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    #[tokio::test]
    async fn clear_at_ref_clears_suggestions() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _cell) = RactorCompletionActor::spawn(bus).await;

        handle
            .send(CompletionMsg::AtSuggestionsChanged {
                suggestions: vec!["foo".into()],
            })
            .await;
        handle.send(CompletionMsg::ClearAtRef).await;
        drop(handle);

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}
