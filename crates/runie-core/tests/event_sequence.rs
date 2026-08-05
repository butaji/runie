//! Reproduces the README's `prompt("X")` event sequence.

mod common;

use std::sync::Arc;

use common::{event_kinds, MockStreamFn, TestLoopBuilder};
use runie_core::types::{AgentMessage, UserContent, UserMessage};

#[tokio::test]
async fn prompt_hello_event_order() {
    let mock = Arc::new(MockStreamFn::hello());
    let test = TestLoopBuilder::new(mock).build();

    let prompt = vec![AgentMessage::User(UserMessage {
        content: vec![UserContent::Text { text: "Hi".into() }],
        timestamp: 1,
    })];

    let outcome = test
        .actor
        .prompt(prompt, runie_core::types::AgentContext::default())
        .await
        .unwrap();
    assert_eq!(outcome.len(), 2);

    let events = test.events.lock().clone();
    let kinds = event_kinds(&events);

    // Expected ordering: AgentStart, TurnStart, MessageStart(user),
    // MessageEnd(user), MessageStart(assistant), MessageUpdate x N,
    // MessageEnd(assistant), TurnEnd, AgentEnd.
    assert!(kinds.starts_with(&["AgentStart", "TurnStart", "MessageStart", "MessageEnd"]));
    assert_eq!(kinds[0], "AgentStart");
    assert_eq!(kinds.last(), Some(&"AgentEnd"));

    let turn_ends = kinds.iter().filter(|k| **k == "TurnEnd").count();
    assert_eq!(turn_ends, 1);

    let message_starts = kinds.iter().filter(|k| **k == "MessageStart").count();
    let message_ends = kinds.iter().filter(|k| **k == "MessageEnd").count();
    assert_eq!(message_starts, message_ends);
}
