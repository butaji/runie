//! Verifies follow-up drain triggers an additional turn.

mod common;

use std::sync::Arc;

use common::{event_kinds, MockStreamFn, TestLoopBuilder};
use runie_core::types::{AgentContext, AgentMessage, UserContent, UserMessage};

#[tokio::test]
async fn follow_up_triggers_another_turn() {
    let mock = Arc::new(MockStreamFn::hello());
    let test = TestLoopBuilder::new(mock).build();

    let initial = vec![AgentMessage::User(UserMessage {
        content: vec![UserContent::Text {
            text: "first".into(),
        }],
        timestamp: 1,
    })];

    // Pre-push a follow-up; it should be drained after the initial run.
    test.follow_up
        .push(AgentMessage::User(UserMessage {
            content: vec![UserContent::Text {
                text: "second".into(),
            }],
            timestamp: 2,
        }))
        .await;

    let outcome = test
        .actor
        .prompt(initial, AgentContext::default())
        .await
        .unwrap();
    // Initial user + assistant + follow-up user + follow-up assistant = 4
    assert!(
        outcome.len() >= 4,
        "expected at least 4 new messages, got {}",
        outcome.len()
    );

    let kinds = event_kinds(&test.events.lock());
    let turn_starts = kinds.iter().filter(|k| **k == "TurnStart").count();
    assert_eq!(
        turn_starts, 2,
        "expected 2 TurnStart events (initial + follow-up)"
    );
}
