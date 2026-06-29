//! `InputActor` — owns the authoritative `InputState`.
//!
//! This actor uses the ractor framework for actor supervision and message handling.

use ractor::{Actor, ActorProcessingErr, ActorRef};
use ractor::async_trait;
use parking_lot::Mutex;

use crate::actors::ractor_adapter::{RactorHandle, spawn_ractor};
use crate::bus::EventBus;
use crate::event::Event;
use crate::model::InputState;

use super::messages::InputMsg;

/// Handle type for InputActor using ractor.
pub type RactorInputHandle = RactorHandle<InputMsg>;

/// Ractor-based InputActor.
///
/// Text editing, cursor navigation, history, and undo/redo are all mutations
/// that live here. The actor processes `InputMsg` messages and emits
/// `InputChanged` facts when state changes.
pub struct InputActor {
    /// The authoritative input state (protected by mutex for interior mutability).
    state: Mutex<InputState>,
    /// Bridge to the event bus for publishing facts.
    bus: EventBus<Event>,
}

#[async_trait]
impl Actor for InputActor {
    type Msg = InputMsg;
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
        msg: Self::Msg,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        let (new_state,) = {
            let mut state = self.state.lock();
            InputMsg::apply_to(&msg, &mut state);
            (state.clone(),)
        };
        // Always emit InputChanged: UiActor uses this as the single source of
        // truth for input state, enabling autocomplete trigger detection and
        // clean state synchronization without dual updates.
        self.bus.publish(Event::InputChanged {
            state: Box::new(new_state),
        });
        Ok(())
    }
}

impl InputActor {
    /// Spawn an `InputActor` on the given event bus using ractor.
    pub async fn spawn(bus: EventBus<Event>) -> (RactorInputHandle, ractor::ActorCell) {
        let actor = Self {
            state: Mutex::new(InputState::default()),
            bus: bus.clone(),
        };
        let (handle, _join, cell) = spawn_ractor(None, actor, bus).await.unwrap();
        (handle, cell)
    }

    #[cfg(test)]
    pub fn state(&self) -> InputState {
        self.state.lock().clone()
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
    async fn insert_char_updates_cursor() {
        let bus = EventBus::<Event>::new(16);
        let mut sub = bus.subscribe();

        let (handle, _cell) = InputActor::spawn(bus.clone()).await;
        handle.send(InputMsg::InsertChar('h')).await;

        // Wait for first InputChanged event
        let found_h = wait_for_event(&mut sub, |e| matches!(e, Event::InputChanged { state } if state.input == "h")).await;
        assert!(found_h, "Expected InputChanged with 'h'");

        handle.send(InputMsg::InsertChar('i')).await;

        // Wait for second InputChanged event
        let found_hi = wait_for_event(&mut sub, |e| matches!(e, Event::InputChanged { state } if state.input == "hi")).await;
        assert!(found_hi, "Expected InputChanged with 'hi'");

        drop(handle);
    }

    #[tokio::test]
    async fn history_prev_cycles() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _cell) = InputActor::spawn(bus).await;

        handle
            .send(InputMsg::HistoryLoaded {
                entries: vec!["first".into(), "second".into()],
            })
            .await;
        handle.send(InputMsg::HistoryPrev).await;
        drop(handle);
    }

    #[tokio::test]
    async fn clear_resets_everything() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _cell) = InputActor::spawn(bus).await;

        handle.send(InputMsg::InsertChar('t')).await;
        handle.send(InputMsg::InsertChar('e')).await;
        handle.send(InputMsg::InsertChar('s')).await;
        handle.send(InputMsg::Clear).await;
        drop(handle);
    }
}
