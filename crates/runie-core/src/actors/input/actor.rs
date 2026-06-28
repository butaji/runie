//! `InputActor` — owns the authoritative `InputState`.
//!
//! This actor uses the ractor framework for actor supervision and message handling.

use ractor::{Actor, ActorProcessingErr, ActorRef};
use ractor::async_trait;
use parking_lot::Mutex;

use crate::actors::ractor_adapter::{EventBusBridge, RactorHandle, spawn_ractor};
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
    bus_bridge: EventBusBridge<Event>,
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
        let (new_state, emit) = {
            let mut state = self.state.lock();
            // Delegate to the apply_to method which handles all state mutations
            InputMsg::apply_to(&msg, &mut state);
            let should_emit = true;
            (state.clone(), should_emit)
        };
        if emit {
            self.bus_bridge.publish(Event::InputChanged {
                state: Box::new(new_state),
            });
        }
        Ok(())
    }
}

impl InputActor {
    /// Spawn an `InputActor` on the given event bus using ractor.
    pub async fn spawn(bus: EventBus<Event>) -> (RactorInputHandle, ractor::ActorCell) {
        let actor = Self {
            state: Mutex::new(InputState::default()),
            bus_bridge: EventBusBridge::new(bus.clone()),
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

    #[tokio::test]
    async fn insert_char_updates_cursor() {
        let bus = EventBus::<Event>::new(16);
        let mut sub = bus.subscribe();

        let (handle, _cell) = InputActor::spawn(bus.clone()).await;
        handle.send(InputMsg::InsertChar('h')).await;
        handle.send(InputMsg::InsertChar('i')).await;
        drop(handle);

        // Give the actor time to process
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let mut events = Vec::new();
        while let Ok(e) = sub.try_recv() {
            if matches!(e, Event::InputChanged { .. }) {
                events.push(e);
            }
        }

        assert_eq!(events.len(), 2);
        if let Event::InputChanged { state } = &events[0] {
            assert_eq!(state.input, "h");
            assert_eq!(state.cursor_pos, 1);
        }
        if let Event::InputChanged { state } = &events[1] {
            assert_eq!(state.input, "hi");
            assert_eq!(state.cursor_pos, 2);
        }
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
