use super::ScrollbackActor;
use crate::widgets::{Line, LineKind, ScrollbackMsg};
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

#[tokio::test]
async fn shared_model_snapshot_reuses_immutable_projection() {
    let actor = ScrollbackActor::new();
    actor
        .apply(ScrollbackMsg::Append(Line::new(LineKind::User, "hello")))
        .await;

    let first = actor.shared_snapshot();
    let second = actor.shared_snapshot();
    assert_eq!(first.lines.len(), 1);
    assert_eq!(first.strong_count(), 3);
    assert_eq!(first, second);
}
