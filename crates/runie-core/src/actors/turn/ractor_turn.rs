//! Ractor-based TurnActor implementation.
//!
//! This module re-exports the actor implementation and hosts the test suite.
//! Implementation details are split into focused modules:
//! - `handlers.rs` — all message handler functions
//! - `actor.rs` — the Actor trait impl and spawn function
//! - `state.rs` — TurnState struct
//! - `types.rs` — TurnActorState and RactorTurnHandle

// Re-export the actor for backward compatibility.
pub use crate::actors::turn::actor::RactorTurnActor;

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::actors::turn::messages::MessageSource;
    use crate::bus::EventBus;
    use crate::Event;

    #[tokio::test]
    async fn run_if_queued_starts_turn() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _, _) = RactorTurnActor::spawn(bus.clone()).await.unwrap();
        let mut sub = bus.subscribe();
        handle
            .send(TurnMsg::SubmitUserMessage {
                content: "hello".into(),
                id: "req.0".into(),
                source: MessageSource::Fresh,
            })
            .await;
        handle.send(TurnMsg::RunIfQueued).await;
        let mut found = false;
        while let Ok(evt) = sub.recv().await {
            if matches!(evt, Event::TurnStarted { .. }) {
                found = true;
                break;
            }
        }
        assert!(found);
    }

    #[tokio::test]
    async fn abort_turn_clears_state() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _, _) = RactorTurnActor::spawn(bus.clone()).await.unwrap();
        let mut sub = bus.subscribe();
        handle
            .send(TurnMsg::SubmitUserMessage {
                content: "hello".into(),
                id: "req.0".into(),
                source: MessageSource::Fresh,
            })
            .await;
        handle.send(TurnMsg::RunIfQueued).await;
        handle.send(TurnMsg::AbortTurn).await;
        let mut found = false;
        while let Ok(evt) = sub.recv().await {
            if matches!(evt, Event::TurnAborted) {
                found = true;
                break;
            }
        }
        assert!(found);
    }

    #[tokio::test]
    async fn error_emits_turned_errored() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _, _) = RactorTurnActor::spawn(bus.clone()).await.unwrap();
        let mut sub = bus.subscribe();
        handle
            .send(TurnMsg::Error {
                id: "req.0".into(),
                message: "oops".into(),
            })
            .await;
        let mut found = false;
        while let Ok(evt) = sub.recv().await {
            if matches!(evt, Event::TurnErrored { .. }) {
                found = true;
                break;
            }
        }
        assert!(found);
    }

    /// Regression test: QueueFollowUp puts in message_queue; DeliverQueued moves it to
    /// request_queue (via FollowUpDelivered); RunIfQueued starts the next turn.
    #[tokio::test]
    async fn queue_follow_up_after_done_starts_queued_turn() {
        use crate::model::DeliveryMode;
        let bus = EventBus::<Event>::new(16);
        let (handle, _, _) = RactorTurnActor::spawn(bus.clone()).await.unwrap();
        let mut sub = bus.subscribe();

        // First turn starts (fresh submit)
        handle
            .send(TurnMsg::SubmitUserMessage {
                content: "first".into(),
                id: "req.0".into(),
                source: MessageSource::Fresh,
            })
            .await;
        handle.send(TurnMsg::RunIfQueued).await;

        // Wait for TurnStarted
        let mut found_first_turn = false;
        while let Ok(evt) = sub.recv().await {
            if matches!(evt, Event::TurnStarted { id, .. } if id == "req.0") {
                found_first_turn = true;
                break;
            }
        }
        assert!(found_first_turn, "First turn should start");

        // Queue a follow-up while first turn is active
        handle
            .send(TurnMsg::QueueFollowUp {
                content: "second".into(),
            })
            .await;

        // First turn completes
        handle
            .send(TurnMsg::Done { id: "req.0".into() })
            .await;

        // Wait for TurnCompleted
        let mut found_completed = false;
        while let Ok(evt) = sub.recv().await {
            if matches!(evt, Event::TurnCompleted) {
                found_completed = true;
                break;
            }
        }
        assert!(found_completed, "First turn should complete");

        // After TurnCompleted, DeliverQueued moves message from message_queue to request_queue.
        use ractor::rpc::CallResult;
        let result = handle
            .inner
            .call(
                |tx| TurnMsg::DeliverQueued {
                    steering_mode: DeliveryMode::OneAtATime,
                    follow_up_mode: DeliveryMode::All,
                    reply: Some(tx),
                },
                Some(std::time::Duration::from_secs(2)),
            )
            .await;
        match result {
            Ok(CallResult::Success(Some(_))) => {}
            Ok(CallResult::Success(None)) => {
                panic!("DeliverQueued should have delivered the follow-up")
            }
            Ok(CallResult::SenderError) => panic!("Sender error on DeliverQueued"),
            Ok(CallResult::Timeout) => panic!("DeliverQueued RPC timed out"),
            Err(e) => panic!("DeliverQueued RPC failed: {:?}", e),
        }

        // RunIfQueued starts the queued turn
        handle.send(TurnMsg::RunIfQueued).await;

        // After RunIfQueued, second turn should start
        let result = tokio::time::timeout(std::time::Duration::from_secs(2), async {
            while let Ok(evt) = sub.recv().await {
                if matches!(evt, Event::TurnStarted { id, .. } if id == "req.1") {
                    return;
                }
            }
        })
        .await;
        assert!(result.is_ok(), "Second turn should start after DeliverQueued + RunIfQueued");
    }

    /// Verify that `#[instrument]` does not panic and handlers process messages normally.
    #[tokio::test]
    async fn turn_actor_handler_runs_with_tracing() {
        use tracing_subscriber::{fmt, prelude::*, EnvFilter};

        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info"));
        let _ = tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().with_target(false).without_time())
            .try_init();

        let bus = EventBus::<Event>::new(16);
        let (handle, _, _) = RactorTurnActor::spawn(bus.clone()).await.unwrap();

        handle
            .send(TurnMsg::SubmitUserMessage {
                content: "hello".into(),
                id: "req.0".into(),
                source: MessageSource::Fresh,
            })
            .await;

        let mut sub = bus.subscribe();
        let found = tokio::time::timeout(std::time::Duration::from_secs(1), async {
            while let Ok(evt) = sub.recv().await {
                if matches!(evt, Event::UserMessageSubmitted { .. }) {
                    return true;
                }
            }
            false
        })
        .await;
        assert!(found.unwrap_or(false), "Handler should process message successfully with tracing");
    }
}
