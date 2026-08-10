use super::ScrollbackActor;
use runie_tui_model::{ScrollbackEvent, ScrollbackLifecycleEvent};

#[tokio::test]
async fn grouped_events_use_actor_mailbox() {
    let actor = ScrollbackActor::new();
    actor
        .apply_grouped(ScrollbackEvent::Lifecycle(
            ScrollbackLifecycleEvent::TurnStarted,
        ))
        .await;
    assert!(actor.model_snapshot().facts.turn_started);
}
