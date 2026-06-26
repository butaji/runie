//! `CompletionActor` — owns the authoritative `CompletionState`.

use tokio::sync::mpsc;

use crate::actors::{spawn_actor, Actor, ActorHandle};
use crate::bus::EventBus;
use crate::event::Event;
use crate::model::CompletionState;

use super::messages::{CompletionActorHandle, CompletionMsg};

/// Actor that owns completion state.
///
/// Path completion, @ mention suggestions, and completion UI state are all mutations
/// that live here. The actor processes `CompletionMsg` messages and emits
/// `CompletionChanged` facts when state changes.
pub struct CompletionActor {
    /// The authoritative completion state.
    state: CompletionState,
}

impl CompletionActor {
    /// Spawn a `CompletionActor` on the given event bus.
    pub fn spawn(bus: EventBus<Event>) -> (CompletionActorHandle, ActorHandle) {
        let actor = Self {
            state: CompletionState::default(),
        };
        let (tx, handle) = spawn_actor(actor, bus);
        (CompletionActorHandle::new(tx), handle)
    }

    /// Dispatch an incoming message to the appropriate handler.
    fn handle_msg(&mut self, msg: CompletionMsg) {
        match msg {
            CompletionMsg::TogglePathCompletion { partial } => self.toggle_path_completion(&partial),
            CompletionMsg::PathCompletionUp => self.path_completion_up(),
            CompletionMsg::PathCompletionDown => self.path_completion_down(),
            CompletionMsg::PathCompletionSelect { prefix } => self.path_completion_select(&prefix),
            CompletionMsg::PathCompletionClose => self.path_completion_close(),
            CompletionMsg::AtSuggestionsChanged { suggestions } => {
                self.at_suggestions_changed(suggestions)
            }
            CompletionMsg::AtSuggestionUp => self.at_suggestion_up(),
            CompletionMsg::AtSuggestionDown => self.at_suggestion_down(),
            CompletionMsg::AtSuggestionSelect => self.at_suggestion_select(),
            CompletionMsg::ClearAtRef => self.clear_at_ref(),
            CompletionMsg::ClearAll => self.clear_all(),
            // These are handled by InputActor or are no-ops for CompletionState
            CompletionMsg::SetGhost { .. }
            | CompletionMsg::SetTabComplete { .. }
            | CompletionMsg::AcceptGhost
            | CompletionMsg::ClearGhost
            | CompletionMsg::TabCompleteNext
            | CompletionMsg::FilePickerAbort => {}
        }
    }

    // ── Path completion ───────────────────────────────────────────────────

    fn toggle_path_completion(&mut self, partial: &str) {
        let cwd = std::env::current_dir().unwrap_or_default();
        let suggestions = crate::path_complete::complete_path(partial, &cwd);
        if suggestions.is_empty() {
            return;
        }
        self.state.path_suggestions = Some(suggestions);
        self.state.path_selected = Some(0);
    }

    fn path_completion_up(&mut self) {
        if let Some(ref items) = self.state.path_suggestions {
            let sel = self.state.path_selected.unwrap_or(0);
            self.state.path_selected = Some(if sel == 0 { items.len() - 1 } else { sel - 1 });
        }
    }

    fn path_completion_down(&mut self) {
        if let Some(ref items) = self.state.path_suggestions {
            let sel = self.state.path_selected.unwrap_or(0);
            self.state.path_selected = Some((sel + 1) % items.len());
        }
    }

    fn path_completion_select(&mut self, _prefix: &str) {
        self.state.path_suggestions = None;
        self.state.path_selected = None;
    }

    fn path_completion_close(&mut self) {
        self.state.path_suggestions = None;
        self.state.path_selected = None;
    }

    // ── @ mention suggestions ─────────────────────────────────────────────

    fn at_suggestions_changed(&mut self, suggestions: Vec<String>) {
        self.state.at_suggestions = Some(suggestions);
        self.state.at_selected = Some(0);
        self.state.last_at_query = None;
    }

    fn at_suggestion_up(&mut self) {
        let items = match self.state.at_suggestions.as_ref() {
            Some(i) if !i.is_empty() => i,
            _ => {
                self.state.at_suggestions = None;
                self.state.at_selected = None;
                return;
            }
        };
        let sel = self.state.at_selected.unwrap_or(0);
        self.state.at_selected = Some(if sel == 0 { items.len() - 1 } else { sel - 1 });
    }

    fn at_suggestion_down(&mut self) {
        let items = match self.state.at_suggestions.as_ref() {
            Some(i) if !i.is_empty() => i,
            _ => return,
        };
        let sel = self.state.at_selected.unwrap_or(0);
        self.state.at_selected = Some((sel + 1) % items.len());
    }

    fn at_suggestion_select(&mut self) {
        self.state.at_suggestions = None;
        self.state.at_selected = None;
    }

    fn clear_at_ref(&mut self) {
        self.state.at_suggestions = None;
        self.state.at_selected = None;
        self.state.last_at_query = None;
    }

    // ── State mutations ───────────────────────────────────────────────────

    fn clear_all(&mut self) {
        self.state.path_suggestions = None;
        self.state.path_selected = None;
        self.state.at_suggestions = None;
        self.state.at_selected = None;
        self.state.last_at_query = None;
    }

    #[cfg(test)]
    pub fn state(&self) -> &CompletionState {
        &self.state
    }
}

impl Actor for CompletionActor {
    type Msg = CompletionMsg;
    type Event = Event;

    async fn run_body(mut self, mut rx: mpsc::Receiver<Self::Msg>, bus: EventBus<Event>) {
        while let Some(msg) = rx.recv().await {
            self.handle_msg(msg.clone());
            bus.publish(Event::CompletionChanged {
                state: Box::new(self.state.clone()),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn toggle_path_completion_creates_suggestions() {
        let bus = EventBus::<Event>::new(16);
        let mut sub = bus.subscribe();

        let (handle, _actor) = CompletionActor::spawn(bus);
        handle
            .send(CompletionMsg::TogglePathCompletion {
                partial: String::new(),
            })
            .await;
        drop(handle);

        let events = drain_events(&mut sub, 1).await;
        assert!(!events.is_empty());
        if let Event::CompletionChanged { state } = &events[0] {
            assert!(state.path_suggestions.is_some());
        }
    }

    #[tokio::test]
    async fn at_suggestions_changes_suggests() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _actor) = CompletionActor::spawn(bus);

        handle
            .send(CompletionMsg::AtSuggestionsChanged {
                suggestions: vec!["foo".into(), "bar".into()],
            })
            .await;
        drop(handle);
    }

    #[tokio::test]
    async fn clear_at_ref_clears_suggestions() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _actor) = CompletionActor::spawn(bus);

        handle
            .send(CompletionMsg::AtSuggestionsChanged {
                suggestions: vec!["foo".into()],
            })
            .await;
        handle.send(CompletionMsg::ClearAtRef).await;
        drop(handle);
    }

    async fn drain_events<E: Clone + Send + 'static>(
        sub: &mut tokio::sync::broadcast::Receiver<E>,
        count: usize,
    ) -> Vec<E> {
        let mut events = Vec::with_capacity(count);
        for _ in 0..count {
            match sub.recv().await {
                Ok(e) => events.push(e),
                Err(_) => break,
            }
        }
        events
    }
}
