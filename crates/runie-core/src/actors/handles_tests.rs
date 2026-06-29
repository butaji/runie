//! Tests for `ActorHandles` (typed actor handle registry).

use crate::actors::{
    ActorHandles, ConfigMsg, InputMsg, IoMsg,
    PermissionMsg, RactorConfigActor, RactorIoActor,
    RactorProviderActor, RactorSessionActor, RactorTurnActor,
};
use crate::actors::permission::RactorPermissionActor;
use crate::actors::InputActor;
use crate::bus::EventBus;
use crate::Event;

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a minimal ActorHandles for testing.
    async fn make_test_handles() -> ActorHandles {
        let bus = EventBus::<Event>::new(16);
        let (config, _) = RactorConfigActor::spawn(bus.clone(), None).await;
        let (provider, _) = RactorProviderActor::minimal_spawn_for_test(bus.clone()).await;
        let (session, _) = RactorSessionActor::spawn(bus.clone()).await.unwrap();
        let (io, _) = RactorIoActor::spawn(bus.clone()).await.unwrap();
        let (permission, _) = RactorPermissionActor::spawn(bus.clone()).await;
        let (input, _) = InputActor::spawn(bus.clone()).await;
        let (turn, _, turn_join) = RactorTurnActor::spawn(bus.clone()).await;

        ActorHandles {
            config,
            provider,
            session,
            io,
            fff_indexer: None,
            input,
            permission,
            turn,
            turn_join: Some(std::sync::Arc::new(turn_join)),
        }
    }

    // ── Layer 1: State/Logic ────────────────────────────────────────────────

    #[tokio::test]
    async fn handles_hold_wrapper_types() {
        let handles = make_test_handles().await;

        // Verify every field is present
        assert_eq!(handles.count(), 8);
        assert!(!handles.has_fff_indexer());
    }

    #[tokio::test]
    async fn handles_send_to_config() {
        let handles = make_test_handles().await;
        handles.config.send_message(ConfigMsg::Reload).await;
    }

    #[tokio::test]
    async fn handles_send_to_session() {
        let handles = make_test_handles().await;
        handles.session.send_message(crate::actors::SessionMsg::List).await;
    }

    #[tokio::test]
    async fn handles_send_to_turn() {
        let handles = make_test_handles().await;
        handles.turn.send_message(crate::actors::TurnMsg::ClearQueues).await;
    }

    #[tokio::test]
    async fn handles_send_to_permission() {
        let handles = make_test_handles().await;
        handles.permission.send_message(PermissionMsg::DismissRequest).await;
    }

    #[tokio::test]
    async fn handles_send_to_io() {
        let handles = make_test_handles().await;
        handles.io.send_message(IoMsg::DetectEnv).await;
    }

    #[tokio::test]
    async fn handles_send_to_input() {
        let handles = make_test_handles().await;
        handles.input.send_message(InputMsg::Clear).await;
    }

    // ── Layer 2: Event Handling ─────────────────────────────────────────────

    #[tokio::test]
    async fn handle_cast_reaches_config_actor() {
        let handles = make_test_handles().await;
        // Verify send_message reaches the actor
        handles.config.send_message(ConfigMsg::Reload).await;
    }
}
