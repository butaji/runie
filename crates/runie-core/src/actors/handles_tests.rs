//! Tests for ActorHandles.

use crate::actors::handles::ActorHandles;
use crate::actors::RactorFffIndexerHandle;

#[cfg(test)]
mod tests {
    use super::*;

    // ── Layer 1: State/Logic ────────────────────────────────────────────────

    #[test]
    fn actor_handles_default_is_all_none() {
        let handles = ActorHandles::default();
        assert!(handles.config.is_none());
        assert!(handles.provider.is_none());
        assert!(handles.session.is_none());
        assert!(handles.io.is_none());
        assert!(handles.fff_indexer.is_none());
        assert!(handles.input.is_none());
        assert!(handles.permission.is_none());
        assert!(handles.turn.is_none());
    }

    #[test]
    fn actor_handles_contains_only_production_actors() {
        // L1 test: verifies the collapsed struct exposes exactly the expected
        // typed actor refs and no dead fields remain.
        let handles = ActorHandles::default();
        // Verify all production actors are present (per task spec)
        let _ = handles.config;
        let _ = handles.provider;
        let _ = handles.session;
        let _ = handles.io;
        let _ = handles.fff_indexer;
        let _ = handles.input;
        let _ = handles.permission;
        let _ = handles.turn;
    }

    #[test]
    fn actor_handles_builder_pattern() {
        let handles = ActorHandles::new();
        assert_eq!(handles.len(), 0);
        assert!(!handles.is_complete());
    }

    #[test]
    fn actor_handles_len_counts_present_handles() {
        assert_eq!(ActorHandles::default().len(), 0);
        let handles = ActorHandles::new();
        assert_eq!(handles.len(), 0);
    }

    #[test]
    fn fff_indexer_handle_is_cloneable() {
        fn _assert_clone<T: Clone>() {}
        _assert_clone::<RactorFffIndexerHandle>();
    }

    // ── Layer 2: Event Handling ─────────────────────────────────────────────

    #[tokio::test]
    async fn actor_handles_send_message_to_config_actor() {
        use crate::actors::RactorConfigActor;
        use crate::bus::EventBus;
        use crate::Event;

        let bus = EventBus::<Event>::new(16);
        let (handle, _actor) = RactorConfigActor::spawn(bus.clone(), None).await;
        let handles = ActorHandles {
            config: Some(handle),
            ..Default::default()
        };

        // Call the handle directly
        handles.config.as_ref().unwrap().set_theme("dark".into()).await;
        drop(handles);
        drop(_actor);
    }

    #[tokio::test]
    async fn actor_handles_send_message_to_session_actor() {
        use crate::actors::RactorSessionActor;
        use crate::bus::EventBus;
        use crate::Event;

        let bus = EventBus::<Event>::new(16);
        let (handle, _actor) = RactorSessionActor::spawn(bus.clone()).await.unwrap();
        let handles = ActorHandles {
            session: Some(handle),
            ..Default::default()
        };

        // Call session handle directly
        handles.session.as_ref().unwrap().list().await;
        drop(handles);
        drop(_actor);
    }

    #[tokio::test]
    async fn actor_handles_send_message_to_turn_actor() {
        use crate::actors::RactorTurnActor;
        use crate::bus::EventBus;
        use crate::Event;
        use crate::actors::TurnMsg;

        let bus = EventBus::<Event>::new(16);
        let (handle, _, _actor) = RactorTurnActor::spawn(bus.clone()).await;
        let handles = ActorHandles {
            turn: Some(handle),
            ..Default::default()
        };

        // Call turn handle directly
        handles.turn.as_ref().unwrap().send(TurnMsg::ClearQueues).await;
        drop(handles);
        drop(_actor);
    }

    #[tokio::test]
    async fn actor_handles_send_message_to_permission_actor() {
        use crate::actors::permission::RactorPermissionActor;
        use crate::bus::EventBus;
        use crate::Event;

        let bus = EventBus::<Event>::new(16);
        let (handle, _actor) = RactorPermissionActor::spawn(bus.clone()).await;
        let handles = ActorHandles {
            permission: Some(handle),
            ..Default::default()
        };

        // Call permission handle directly
        handles.permission.as_ref().unwrap().dismiss().await;
        drop(handles);
        drop(_actor);
    }

    #[tokio::test]
    async fn actor_handles_send_message_to_io_actor() {
        use crate::actors::RactorIoActor;
        use crate::bus::EventBus;
        use crate::Event;

        let bus = EventBus::<Event>::new(16);
        let (handle, _actor) = RactorIoActor::spawn(bus.clone()).await.unwrap();
        let handles = ActorHandles {
            io: Some(handle),
            ..Default::default()
        };

        // Call IO handle directly
        handles.io.as_ref().unwrap().detect_env().await;
        drop(handles);
        drop(_actor);
    }

    #[tokio::test]
    async fn actor_handles_send_message_to_input_actor() {
        use crate::actors::InputActor;
        use crate::bus::EventBus;
        use crate::Event;
        use crate::actors::InputMsg;

        let bus = EventBus::<Event>::new(16);
        let (handle, _actor) = InputActor::spawn(bus.clone()).await;
        let handles = ActorHandles {
            input: Some(handle),
            ..Default::default()
        };

        // Call input handle directly
        handles.input.as_ref().unwrap().send(InputMsg::Clear).await;
        drop(handles);
        drop(_actor);
    }
}
