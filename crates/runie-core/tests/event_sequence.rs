//! Reproduces the README's `prompt("X")` event sequence.

mod common;

use std::sync::Arc;

use common::{event_kinds, MockStreamFn, TestLoopBuilder};
use runie_core::types::{
    AgentContext, AgentMessage, AssistantMessage, StopReason, UserContent, UserMessage,
};

/// Extract the assistant message from a `MessageStart`/`MessageEnd` event.
fn assistant_of(event: &runie_core::types::AgentEvent) -> Option<&AssistantMessage> {
    match event {
        runie_core::types::AgentEvent::MessageStart {
            message: AgentMessage::Assistant(a),
        }
        | runie_core::types::AgentEvent::MessageEnd {
            message: AgentMessage::Assistant(a),
        } => Some(a),
        _ => None,
    }
}

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

#[tokio::test]
async fn steering_pushed_before_submit_is_injected_before_first_assistant() {
    let mock = Arc::new(MockStreamFn::hello());
    let test = TestLoopBuilder::new(mock).build();

    // A steering message queued before the prompt is submitted must be
    // injected before the first assistant response, within the first turn
    // (pi parity: "the user may have typed while waiting").
    test.steering
        .push(AgentMessage::User(UserMessage {
            content: vec![UserContent::Text {
                text: "steer".into(),
            }],
            timestamp: 2,
        }))
        .await;

    let prompt = vec![AgentMessage::User(UserMessage {
        content: vec![UserContent::Text { text: "Hi".into() }],
        timestamp: 1,
    })];

    let outcome = test
        .actor
        .prompt(prompt, AgentContext::default())
        .await
        .unwrap();
    // prompt user + steering user + assistant = 3 new messages.
    assert_eq!(outcome.len(), 3);
    assert!(outcome.iter().any(|m| matches!(m,
        AgentMessage::User(u) if u.content.iter().any(|c| matches!(c, UserContent::Text { text } if text == "steer")))));

    let kinds = event_kinds(&test.events.lock());
    // Exactly one turn: steering is injected inside the first turn.
    let turn_starts = kinds.iter().filter(|k| **k == "TurnStart").count();
    assert_eq!(turn_starts, 1);

    // The steering user message must be emitted before the assistant message.
    let events = test.events.lock();
    let steer_idx = events.iter().position(|e| {
        matches!(e,
        runie_core::types::AgentEvent::MessageStart { message: AgentMessage::User(u) }
            if u.content.iter().any(|c| matches!(c, UserContent::Text { text } if text == "steer")))
    });
    let assistant_idx = events.iter().position(|e| {
        matches!(
            e,
            runie_core::types::AgentEvent::MessageStart {
                message: AgentMessage::Assistant(_)
            }
        )
    });
    let steer_idx = steer_idx.expect("steering message should be emitted");
    let assistant_idx = assistant_idx.expect("assistant message should be emitted");
    assert!(
        steer_idx < assistant_idx,
        "steering ({steer_idx}) should precede assistant ({assistant_idx})"
    );
}

#[tokio::test]
async fn streaming_partial_starts_pending_and_ends_with_final_reason() {
    let mock = Arc::new(MockStreamFn::hello());
    let test = TestLoopBuilder::new(mock).build();

    let prompt = vec![AgentMessage::User(UserMessage {
        content: vec![UserContent::Text { text: "Hi".into() }],
        timestamp: 1,
    })];
    test.actor
        .prompt(prompt, AgentContext::default())
        .await
        .unwrap();

    let events = test.events.lock();
    // The assistant message_start carries stop_reason Pending (pi proxy.ts:124).
    let start = events
        .iter()
        .find_map(assistant_of)
        .expect("assistant message should be emitted");
    assert_eq!(
        start.stop_reason,
        Some(StopReason::Pending),
        "streaming partial should begin Pending"
    );
    // The final message_end carries the real stop reason.
    let ends: Vec<_> = events.iter().filter_map(assistant_of).collect();
    if let Some(final_a) = ends.last() {
        assert_eq!(final_a.stop_reason, Some(StopReason::Stop));
    }
}
