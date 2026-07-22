#![allow(clippy::too_many_lines)]

use crate::model::state::AgentState;
use crate::model::{AppState, ChatMessage, Role};
use crate::Event;

#[test]
fn queue_empty_by_default() {
    let state = AppState::default();
    assert!(state.agent.message_queue.is_empty());
}

#[test]
fn submit_during_turn_queues_steering() {
    let mut state = AppState::default();
    state.agent.turn_active = true;

    state.update(crate::Event::Input('h'));
    state.update(crate::Event::Input('i'));
    state.update(Event::submit());
    assert_eq!(state.agent.message_queue.len(), 1);
    assert_eq!(state.agent.message_queue[0].content, "hi");
    assert!(matches!(
        state.agent.message_queue[0].kind,
        crate::model::QueuedMessageKind::Steering
    ));
    assert!(state.input.input.is_empty());
}

#[test]
fn submit_when_idle_sends_immediately() {
    let mut state = AppState::default();
    state.agent.turn_active = false;

    state.update(crate::Event::Input('h'));
    state.update(crate::Event::Input('i'));
    state.update(Event::submit());
    assert!(state.agent.message_queue.is_empty());
    assert_eq!(state.session.messages.len(), 1);
    assert_eq!(state.session.messages[0].role, Role::User);
    assert_eq!(state.session.messages[0].content(), "hi");
}

#[test]
fn follow_up_queues_for_later() {
    let mut state = AppState::default();
    state.agent.turn_active = true;

    state.update(crate::Event::Input('b'));
    state.update(crate::Event::Input('y'));
    state.update(crate::Event::FollowUp);
    assert_eq!(state.agent.message_queue.len(), 1);
    assert!(matches!(
        state.agent.message_queue[0].kind,
        crate::model::QueuedMessageKind::FollowUp
    ));
}

#[test]
fn deliver_queue_at_turn_end() {
    let mut state = AppState::default();
    state.agent.turn_active = true;

    state.update(crate::Event::Input('h'));
    state.update(crate::Event::Input('i'));
    state.update(Event::submit());
    assert_eq!(state.agent.message_queue.len(), 1);

    state.update(crate::Event::Done { id: "req.0".to_string() });
    assert!(state.agent.message_queue.is_empty());
    assert_eq!(state.session.messages.len(), 1);
    assert_eq!(state.session.messages[0].content(), "hi");
}

#[test]
fn queue_multiple_messages() {
    let mut state = AppState::default();
    state.agent.turn_active = true;

    state.update(crate::Event::Input('a'));
    state.update(Event::submit());
    state.update(crate::Event::Input('b'));
    state.update(crate::Event::FollowUp);
    assert_eq!(state.agent.message_queue.len(), 2);
    assert!(matches!(
        state.agent.message_queue[0].kind,
        crate::model::QueuedMessageKind::Steering
    ));
    assert!(matches!(
        state.agent.message_queue[1].kind,
        crate::model::QueuedMessageKind::FollowUp
    ));
}

#[test]
fn escape_clears_queue_and_restores() {
    let mut state = AppState::default();
    state.agent.turn_active = true;

    state.update(crate::Event::Input('h'));
    state.update(crate::Event::Input('i'));
    state.update(Event::submit());
    assert_eq!(state.agent.message_queue.len(), 1);

    state.update(crate::Event::Abort);
    assert!(state.agent.message_queue.is_empty());
    assert_eq!(state.input.input, "hi");
}

#[test]
fn steering_delivered_before_follow_up() {
    let mut state = AppState::default();
    state.agent.turn_active = true;

    state.update(crate::Event::Input('s'));
    state.update(Event::submit());
    state.update(crate::Event::Input('f'));
    state.update(crate::Event::FollowUp);

    state.update(crate::Event::Done { id: "req.0".to_string() });
    let user_msgs: Vec<&ChatMessage> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::User)
        .collect();
    assert_eq!(user_msgs.len(), 1);
    assert_eq!(user_msgs[0].content(), "s");

    state.update(crate::Event::Done { id: "req.1".to_string() });
    let user_msgs: Vec<&ChatMessage> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::User)
        .collect();
    assert_eq!(user_msgs.len(), 2);
    assert_eq!(user_msgs[1].content(), "f");
}

// Delivery mode tests

#[test]
fn delivery_mode_defaults_to_one_at_a_time() {
    let state = AppState::default();
    assert!(matches!(
        state.config.steering_mode,
        crate::model::DeliveryMode::OneAtATime
    ));
    assert!(matches!(
        state.config.follow_up_mode,
        crate::model::DeliveryMode::OneAtATime
    ));
}

#[test]
fn steering_mode_all_batches_messages() {
    use crate::model::DeliveryMode;
    let mut state = AppState::default();
    state.agent.turn_active = true;

    state.config.steering_mode = DeliveryMode::All;

    // Queue three steering messages
    state.update(crate::Event::Input('a'));
    state.update(Event::submit());
    state.update(crate::Event::Input('b'));
    state.update(Event::submit());
    state.update(crate::Event::Input('c'));
    state.update(Event::submit());

    assert_eq!(state.agent.message_queue.len(), 3);

    // Trigger delivery
    state.update(crate::Event::Done { id: "req.0".to_string() });

    // All three should be batched into one request
    assert!(state.agent.message_queue.is_empty());
    assert_eq!(state.agent.request_queue.len(), 1);
    let (content, _) = &state.agent.request_queue[0];
    assert_eq!(content, "a\nb\nc");
}

#[test]
fn follow_up_mode_all_batches_messages() {
    use crate::model::DeliveryMode;
    let mut state = AppState::default();
    state.agent.turn_active = true;

    state.config.follow_up_mode = DeliveryMode::All;
    // Queue three follow-up messages
    state.update(crate::Event::Input('x'));
    state.update(crate::Event::FollowUp);
    state.update(crate::Event::Input('y'));
    state.update(crate::Event::FollowUp);
    state.update(crate::Event::Input('z'));
    state.update(crate::Event::FollowUp);
    assert_eq!(state.agent.message_queue.len(), 3);
    // First complete a turn to trigger delivery
    state.update(crate::Event::Input('i'));
    state.update(Event::submit());
    // After submit, message_queue should have [x, y, z, "i"]
    assert_eq!(
        state.agent.message_queue.len(),
        4,
        "Expected 4 queued messages before turn done"
    );
    state.update(crate::Event::Done { id: "req.0".to_string() });
    // All three follow-ups should be batched into one request
    assert!(
        state.agent.message_queue.is_empty(),
        "Expected empty queue after turn done, got {:?}",
        state.agent.message_queue
    );
    assert_eq!(state.agent.request_queue.len(), 2); // init + batched follow-ups
    let (_, id) = &state.agent.request_queue[1];
    assert_eq!(id, "req.1"); // The batched request
}

#[test]
fn one_at_a_time_delivers_separately() {
    use crate::model::DeliveryMode;
    let mut state = AppState::default();
    state.agent.turn_active = true;

    state.config.steering_mode = DeliveryMode::OneAtATime;

    // Queue three steering messages
    state.update(crate::Event::Input('a'));
    state.update(Event::submit());
    state.update(crate::Event::Input('b'));
    state.update(Event::submit());
    state.update(crate::Event::Input('c'));
    state.update(Event::submit());

    assert_eq!(state.agent.message_queue.len(), 3);

    // Trigger first delivery
    state.update(crate::Event::Done { id: "req.0".to_string() });

    // Only one should be delivered
    assert_eq!(state.agent.message_queue.len(), 2);
    assert_eq!(state.agent.request_queue.len(), 1);

    // Second delivery
    state.update(crate::Event::Done { id: "req.1".to_string() });
    assert_eq!(state.agent.message_queue.len(), 1);
    assert_eq!(state.agent.request_queue.len(), 2);
}

#[test]
fn steering_and_follow_up_modes_independent() {
    use crate::model::DeliveryMode;
    let mut state = AppState::default();
    state.agent.turn_active = true;

    state.config.steering_mode = DeliveryMode::All;
    state.config.follow_up_mode = DeliveryMode::OneAtATime;

    // Queue two steering and two follow-up
    state.update(crate::Event::Input('a'));
    state.update(Event::submit());
    state.update(crate::Event::Input('b'));
    state.update(Event::submit());
    state.update(crate::Event::Input('x'));
    state.update(crate::Event::FollowUp);
    state.update(crate::Event::Input('y'));
    state.update(crate::Event::FollowUp);

    // Complete a turn - steering should batch, follow-up should not
    state.update(crate::Event::Done { id: "req.0".to_string() });

    assert_eq!(state.agent.message_queue.len(), 2); // Two follow-ups still queued
    assert_eq!(state.agent.request_queue.len(), 1); // One batched steering
    assert_eq!(state.agent.request_queue[0].0, "a\nb"); // Batched content

    // Complete second turn - first follow-up delivered
    state.update(crate::Event::Done { id: "req.1".to_string() });
    assert_eq!(state.agent.message_queue.len(), 1); // One follow-up left
    assert_eq!(state.agent.request_queue.len(), 2);
}

// Dequeue tests

#[test]
fn dequeue_restores_last() {
    let mut state = AppState::default();
    state.agent.turn_active = true;

    state.update(crate::Event::Input('h'));
    state.update(crate::Event::Input('i'));
    state.update(Event::submit());
    assert_eq!(state.agent.message_queue.len(), 1);

    state.update(crate::Event::Dequeue);
    assert!(state.agent.message_queue.is_empty());
    assert_eq!(state.input.input, "hi");
}

#[test]
fn dequeue_sets_cursor_end() {
    let mut state = AppState::default();
    state.agent.turn_active = true;

    state.update(crate::Event::Input('a'));
    state.update(crate::Event::Input('b'));
    state.update(Event::submit());

    state.update(crate::Event::Dequeue);
    assert_eq!(state.input.cursor_pos, 2);
}

#[test]
fn dequeue_replaces_input() {
    let mut state = AppState::default();
    state.agent.turn_active = true;

    state.update(crate::Event::Input('o'));
    state.update(crate::Event::Input('l'));
    state.update(crate::Event::Input('d'));
    state.update(Event::submit());

    state.update(crate::Event::Input('n'));
    state.update(crate::Event::Input('e'));
    state.update(crate::Event::Input('w'));
    assert_eq!(state.input.input, "new");

    state.update(crate::Event::Dequeue);
    assert_eq!(state.input.input, "old");
}

#[test]
fn dequeue_empty_flashes() {
    let mut state = AppState::default();
    assert!(state.agent.message_queue.is_empty());
    assert_eq!(state.input.input_flash, 0);

    state.update(crate::Event::Dequeue);
    assert_eq!(state.input.input_flash, 3);
}

#[test]
fn dequeue_lifo() {
    let mut state = AppState::default();
    state.agent.turn_active = true;

    state.update(crate::Event::Input('a'));
    state.update(Event::submit());
    state.update(crate::Event::Input('b'));
    state.update(crate::Event::FollowUp);
    assert_eq!(state.agent.message_queue.len(), 2);

    state.update(crate::Event::Dequeue);
    assert_eq!(state.agent.message_queue.len(), 1);
    assert_eq!(state.input.input, "b");

    state.update(crate::Event::Dequeue);
    assert!(state.agent.message_queue.is_empty());
    assert_eq!(state.input.input, "a");
}
// Alt+Enter follow-up queue
#[test]
fn alt_enter_queues_follow_up_while_thinking() {
    let mut state = AppState::default();
    state.agent.turn_active = true;

    for c in "follow up".chars() {
        state.update(crate::Event::Input(c));
    }
    state.update(crate::Event::FollowUp);

    assert_eq!(state.agent.message_queue.len(), 1);
    assert!(matches!(
        state.agent.message_queue[0].kind,
        crate::model::QueuedMessageKind::FollowUp
    ));
    assert_eq!(state.agent.message_queue[0].content, "follow up");
    assert!(state.input.input.is_empty());
}

#[test]
fn alt_enter_queues_input_as_follow_up_when_idle() {
    let mut state = AppState::default();
    state.agent.turn_active = false;

    for c in "hello".chars() {
        state.update(crate::Event::Input(c));
    }
    state.update(crate::Event::FollowUp);

    assert_eq!(state.agent.message_queue.len(), 1);
    assert!(matches!(
        state.agent.message_queue[0].kind,
        crate::model::QueuedMessageKind::FollowUp
    ));
    assert_eq!(state.agent.message_queue[0].content, "hello");
    assert!(state.input.input.is_empty());
}
// Alt+Up dequeue
#[test]
fn alt_up_restores_last_queued_message() {
    let mut state = AppState::default();
    state.agent.turn_active = true;

    for c in "queued".chars() {
        state.update(crate::Event::Input(c));
    }
    state.update(Event::submit());
    assert_eq!(state.agent.message_queue.len(), 1);

    state.update(crate::Event::Dequeue);
    assert!(state.agent.message_queue.is_empty());
    assert_eq!(state.input.input, "queued");
    assert_eq!(state.input.cursor_pos, 6);
}

#[test]
fn alt_up_replaces_current_input_with_queued() {
    let mut state = AppState::default();
    state.agent.turn_active = true;

    for c in "old".chars() {
        state.update(crate::Event::Input(c));
    }
    state.update(Event::submit());

    for c in "new".chars() {
        state.update(crate::Event::Input(c));
    }
    assert_eq!(state.input.input, "new");

    state.update(crate::Event::Dequeue);
    assert_eq!(state.input.input, "old");
}

#[test]
fn alt_up_empty_queue_flashes_input() {
    let mut state = AppState::default();
    assert!(state.agent.message_queue.is_empty());
    assert_eq!(state.input.input_flash, 0);

    state.update(crate::Event::Dequeue);
    assert_eq!(state.input.input_flash, 3);
}
// Abort during streaming
#[test]
fn abort_during_streaming_clears_turn_and_allows_new_submit() {
    let mut state = AppState::default();
    state.config.vim_mode = true;
    state.agent.turn_active = true;

    state.set_streaming(true);
    state.input.input = "hi".into();
    state.update(Event::submit());
    assert_eq!(state.agent.message_queue.len(), 1);

    state.update(crate::Event::Escape);
    assert!(!state.agent.turn_active);
    assert!(!state.agent.streaming);
    assert!(state.agent.message_queue.is_empty());
    assert_eq!(state.input.input, "hi");

    state.input.input = "hi again".into();
    state.update(Event::submit());
    assert!(
        state
            .session
            .messages
            .iter()
            .any(|m| m.content() == "hi again"),
        "New submit should work after abort"
    );
}

#[test]
fn abort_during_streaming_resets_timers() {
    let mut state = AppState::default();
    state.config.vim_mode = true;
    state.agent.turn_active = true;

    state.set_streaming(true);
    state.agent.turn_started_at = Some(std::time::Instant::now());
    state.agent.thinking_started_at = Some(std::time::Instant::now());
    state.agent.tool_started_at = Some(std::time::Instant::now());

    state.update(crate::Event::Escape);

    assert!(state.agent.turn_started_at.is_none());
    assert!(state.agent.thinking_started_at.is_none());
    assert!(state.agent.tool_started_at.is_none());
}

// ── MessageOrigin tests ───────────────────────────────────────────────────────

#[test]
fn steering_message_has_steering_origin() {
    let mut state = AppState::default();
    state.agent.turn_active = true;

    state.update(crate::Event::Input('h'));
    state.update(crate::Event::Input('i'));
    state.update(Event::submit());

    // Complete turn to trigger delivery
    state.update(crate::Event::Done { id: "req.0".to_string() });

    // Dump session messages for debugging
    eprintln!("Session messages: {:?}", state.session.messages);

    // Find the delivered steering message
    let steering_msg = state
        .session
        .messages
        .iter()
        .find(|m| m.content() == "hi" && m.role == Role::User);

    assert!(
        steering_msg.is_some(),
        "Steering message should be in session"
    );
    assert!(matches!(
        steering_msg.unwrap().metadata.origin,
        crate::message::MessageOrigin::Steering
    ));
}

#[test]
fn follow_up_message_has_follow_up_origin() {
    let mut state = AppState::default();
    state.agent.turn_active = true;

    state.update(crate::Event::Input('f'));
    state.update(crate::Event::Input('o'));
    state.update(crate::Event::Input('l'));
    state.update(crate::Event::Input('l'));
    state.update(crate::Event::FollowUp);

    // Complete turn to trigger delivery
    state.update(crate::Event::Done { id: "req.0".to_string() });

    // Dump session messages for debugging
    eprintln!("Session messages: {:?}", state.session.messages);

    // Find the delivered follow-up message
    let follow_up_msg = state
        .session
        .messages
        .iter()
        .find(|m| m.content() == "foll" && m.role == Role::User);

    assert!(
        follow_up_msg.is_some(),
        "Follow-up message should be in session"
    );
    assert!(matches!(
        follow_up_msg.unwrap().metadata.origin,
        crate::message::MessageOrigin::FollowUp
    ));
}

#[test]
fn idle_submit_has_user_origin() {
    let mut state = AppState::default();
    state.agent.turn_active = false;

    state.update(crate::Event::Input('h'));
    state.update(crate::Event::Input('e'));
    state.update(crate::Event::Input('l'));
    state.update(crate::Event::Input('l'));
    state.update(Event::submit());

    // Dump session messages for debugging
    eprintln!("Session messages: {:?}", state.session.messages);

    // Find the user message
    let user_msg = state
        .session
        .messages
        .iter()
        .find(|m| m.content() == "hell" && m.role == Role::User);

    assert!(user_msg.is_some(), "User message should be in session");
    assert!(matches!(
        user_msg.unwrap().metadata.origin,
        crate::message::MessageOrigin::User
    ));
}
