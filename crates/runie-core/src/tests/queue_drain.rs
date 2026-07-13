//! Regression tests for the status-bar "(N queued)" leak.
//!
//! Live-verified bug: `queue_count = message_queue.len() + request_queue.len()`
//! grew by ~1 every turn and never drained. Root cause: the projection pushes
//! to `request_queue` on `UserMessageSubmitted` (and on steering/follow-up
//! delivery), but `TurnStarted` — the fact emitted when the TurnActor pops its
//! own request queue — never popped the projection's queue.
//!
//! These tests drive the exact fact sequence the TurnActor emits in production
//! and assert enqueue/dequeue symmetry: the queue must drain on every turn
//! path, and queued content must be attributed to its own turn.

use crate::model::{AppState, Role};
use crate::Event;

fn queue_count(state: &AppState) -> usize {
    state.agent_state().message_queue.len() + state.agent_state().request_queue.len()
}

/// A plain turn drains the request queue: submit pushes, TurnStarted pops.
#[test]
fn turn_started_drains_request_queue() {
    let mut state = AppState::default();

    state.update(Event::UserMessageSubmitted {
        id: "req.0".to_string(),
        content: "first".to_string(),
    });
    // Between submit and turn start the message is pending: count is 1.
    assert_eq!(queue_count(&state), 1, "submitted message is pending");

    state.update(Event::TurnStarted {
        id: "req.0".to_string(),
        request_id: "req.0".to_string(),
        content: "first".to_string(),
    });
    assert_eq!(
        queue_count(&state),
        0,
        "TurnStarted must pop the projection request_queue (TurnActor already popped its own)"
    );
}

/// The queue count must not accumulate across sequential turns.
///
/// Reproduces the live bug where "(N queued)" grew by one every turn
/// (observed 2, 3, 4 ... 7 across a session with one submission per turn).
#[test]
fn queue_count_does_not_grow_across_turns() {
    let mut state = AppState::default();

    for turn in 0..4 {
        let id = format!("req.{turn}");
        let content = format!("message number {turn}");

        state.update(Event::UserMessageSubmitted {
            id: id.clone(),
            content: content.clone(),
        });
        assert_eq!(
            queue_count(&state),
            1,
            "turn {turn}: exactly one pending message after submit"
        );

        state.update(Event::TurnStarted {
            id: id.clone(),
            request_id: id.clone(),
            content,
        });
        assert_eq!(
            queue_count(&state),
            0,
            "turn {turn}: queue must be empty once the turn starts (was growing by one per turn)"
        );

        // Turn completes: actor emits TurnComplete then TurnCompleted.
        state.update(Event::TurnComplete {
            id: id.clone(),
            duration_secs: 0.1,
        });
        state.update(Event::TurnCompleted);
        assert_eq!(
            queue_count(&state),
            0,
            "turn {turn}: queue must be empty when idle"
        );
    }
}

/// A queued steering message delivered after a turn must drain on its own
/// TurnStarted and be attributed to its own request id — never leak into the
/// count or be delivered as a different turn's user message.
#[test]
fn steering_delivery_drains_and_attributes_to_own_turn() {
    let mut state = AppState::default();

    // Turn 1 runs.
    state.update(Event::UserMessageSubmitted {
        id: "req.0".to_string(),
        content: "first".to_string(),
    });
    state.update(Event::TurnStarted {
        id: "req.0".to_string(),
        request_id: "req.0".to_string(),
        content: "first".to_string(),
    });

    // User types steering while turn 1 is active.
    state.update(Event::QueueSteeringAdded {
        id: "q.steering.0".to_string(),
        content: "steer me".to_string(),
    });
    assert_eq!(queue_count(&state), 1, "steering is visibly queued");

    // Turn 1 completes; the steering message is delivered for the next turn.
    state.update(Event::TurnComplete {
        id: "req.0".to_string(),
        duration_secs: 0.1,
    });
    state.update(Event::TurnCompleted);
    state.update(Event::SteeringDelivered {
        content: "steer me".to_string(),
        id: "req.1".to_string(),
    });
    assert_eq!(
        state.agent_state().message_queue.len(),
        0,
        "delivered steering leaves the message queue"
    );
    assert_eq!(
        queue_count(&state),
        1,
        "delivered steering is pending in the request queue"
    );

    // The next turn starts with the steering content under its own id.
    state.update(Event::TurnStarted {
        id: "req.1".to_string(),
        request_id: "req.1".to_string(),
        content: "steer me".to_string(),
    });
    assert_eq!(
        queue_count(&state),
        0,
        "steering turn start must drain the request queue"
    );

    // Attribution: the steering content is its own user message, not turn 1's.
    let steering_msg = state
        .session()
        .messages
        .iter()
        .find(|m| m.id == "req.1")
        .expect("steering message recorded under its own request id");
    assert_eq!(steering_msg.role, Role::User);
    assert_eq!(steering_msg.content(), "steer me");
    assert!(matches!(
        steering_msg.metadata.origin,
        crate::message::MessageOrigin::Steering
    ));
    let first_msg = state
        .session()
        .messages
        .iter()
        .find(|m| m.id == "req.0" && m.role == Role::User)
        .expect("turn 1's user message still present");
    assert_eq!(
        first_msg.content(),
        "first",
        "turn 1's message must not be rewritten with steering content"
    );
}

/// AppState's `next_id` must never reissue an id already present in the
/// session. Delivered steering/follow-up messages get their ids from the
/// TurnActor's independent counter (and replayed sessions restore messages
/// without advancing AppState's counter), so a fresh submit could otherwise
/// reuse a delivered id.
#[test]
fn next_id_skips_ids_already_in_session() {
    let mut state = AppState::default();
    // Production fact sequence: turn 1 submits req.0, steering is delivered
    // under the TurnActor's own counter as req.1. AppState's session_msg_id
    // counter is never advanced by these facts.
    state.update(Event::UserMessageSubmitted {
        id: "req.0".to_string(),
        content: "first".to_string(),
    });
    state.update(Event::TurnStarted {
        id: "req.0".to_string(),
        request_id: "req.0".to_string(),
        content: "first".to_string(),
    });
    state.update(Event::QueueSteeringAdded {
        id: "q.steering.0".to_string(),
        content: "steer".to_string(),
    });
    state.update(Event::SteeringDelivered {
        content: "steer".to_string(),
        id: "req.1".to_string(),
    });

    let id = state.next_id();
    assert_eq!(
        id, "req.2",
        "next_id must skip req.0/req.1 which already exist in the session"
    );
}

/// End-to-end attribution: a fresh submit after a steering delivery must not
/// be dropped by the idempotency guard in `apply_user_message_submitted`.
/// Before the fix, the colliding id caused the new user message to vanish
/// from the session, so the turn's response was routed to (and rendered as)
/// the older message — the observed "stale response" bug.
#[test]
fn fresh_submit_after_delivery_is_not_dropped() {
    let mut state = AppState::default();
    state.update(Event::UserMessageSubmitted {
        id: "req.0".to_string(),
        content: "first".to_string(),
    });
    state.update(Event::TurnStarted {
        id: "req.0".to_string(),
        request_id: "req.0".to_string(),
        content: "first".to_string(),
    });
    state.update(Event::QueueSteeringAdded {
        id: "q.steering.0".to_string(),
        content: "steer".to_string(),
    });
    state.update(Event::SteeringDelivered {
        content: "steer".to_string(),
        id: "req.1".to_string(),
    });
    state.update(Event::TurnStarted {
        id: "req.1".to_string(),
        request_id: "req.1".to_string(),
        content: "steer".to_string(),
    });
    state.update(Event::TurnComplete {
        id: "req.1".to_string(),
        duration_secs: 0.1,
    });
    state.update(Event::TurnCompleted);

    // Production: submit_user_message generates the id via next_id, then the
    // TurnActor echoes it back in UserMessageSubmitted.
    let id = state.next_id();
    state.update(Event::UserMessageSubmitted {
        id: id.clone(),
        content: "second".to_string(),
    });

    let msg = state
        .session()
        .messages
        .iter()
        .find(|m| m.id == id && m.role == Role::User);
    assert!(
        matches!(msg, Some(m) if m.content() == "second"),
        "fresh submit after a delivery must be recorded, not dropped as a duplicate"
    );
}

/// Direct projection-level symmetry check: `apply_turn_started` pops what
/// `apply_user_message_submitted` pushed.
#[test]
fn apply_turn_started_pops_request_queue_projection() {
    let mut state = AppState::default();
    state.apply_user_message_submitted("req.0".to_string(), "hello".to_string());
    assert_eq!(state.agent_state().request_queue.len(), 1);

    state.apply_turn_started();
    assert!(
        state.agent_state().request_queue.is_empty(),
        "apply_turn_started must mirror the TurnActor's pop_queue"
    );

    // Popping an empty queue is a harmless no-op (defensive against
    // TurnStarted facts without a preceding submit, e.g. in tests).
    state.apply_turn_started();
    assert!(state.agent_state().request_queue.is_empty());
}
