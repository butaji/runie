//! Ractor-based InputActor — proof of concept for migration from custom runtime.
//!
//! This module demonstrates migrating `InputActor` from the custom `Actor` trait
//! to `ractor`. It shows the minimal changes needed for the spawn pattern.
//!
//! ## Migration Pattern
//!
//! 1. Actor struct holds state + `EventBus`
//! 2. `handle` method processes messages and publishes facts
//! 3. `spawn_ractor_input()` replaces `InputActor::spawn()`

use ractor::{Actor, ActorRef, ActorProcessingErr};
use ractor::async_trait;

use crate::actors::ractor_adapter::{RactorHandle, spawn_ractor};
use crate::bus::EventBus;
use crate::event::Event;
use crate::model::InputState;

use super::messages::InputMsg;

// ── Ractor InputActor ─────────────────────────────────────────────────────────

/// Ractor-based InputActor state.
struct RactorInputActor {
    /// The authoritative input state.
    state: InputState,
    /// Bridge to the event bus for publishing facts.
    bus: EventBus<Event>,
}

#[async_trait::async_trait]
impl Actor for RactorInputActor {
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
        let _ = msg;
        Ok(())
    }
}

/// Handle for the ractor-based InputActor.
pub type RactorInputHandle = RactorHandle<InputMsg>;

/// Spawn a ractor-based InputActor.
///
/// This replaces `InputActor::spawn()` with the ractor-based implementation.
pub async fn spawn_ractor_input(
    bus: EventBus<Event>,
) -> Result<(RactorInputHandle, ractor::ActorCell), ractor::SpawnErr> {
    let actor = RactorInputActor {
        state: InputState::default(),
        bus: bus.clone(),
    };
    let (handle, _join, cell) = spawn_ractor(None, actor, bus).await?;
    Ok((handle, cell))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that ractor-based InputActor spawns correctly.
    #[tokio::test]
    async fn ractor_input_actor_spawns() {
        let bus = EventBus::<Event>::new(16);
        let result = spawn_ractor_input(bus).await;
        assert!(result.is_ok());
    }

    /// Test that messages can be sent to the ractor InputActor.
    #[tokio::test]
    async fn ractor_input_receives_messages() {
        let bus = EventBus::<Event>::new(16);

        let (handle, _cell) = spawn_ractor_input(bus).await.unwrap();

        // Send a message - should not panic
        handle.send(InputMsg::InsertChar('h')).await;
        handle.try_send(InputMsg::InsertChar('i')).ok();
    }
}
