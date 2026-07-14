//! Test for duplicate user message bug in the feed.
//!
//! The bug: when a user submits a message like "say hello", it appears TWICE
//! in the feed when using the mock provider.

use crate::model::{AppState, Role};
use crate::view::elements::Element;
use crate::view::LazyCache;
use crate::Event;

/// Test that a single user message submission produces exactly one UserMessage in the feed.
#[test]
fn test_single_user_message_in_feed() {
    let mut state = AppState::default();

    // 1. User submits a message
    state.update(Event::UserMessageSubmitted {
        id: "req.0".to_string(),
        content: "say hello".to_string(),
    });

    // 2. Turn starts
    state.update(Event::TurnStarted {
        id: "req.0".to_string(),
        request_id: "req.0".to_string(),
        content: "say hello".to_string(),
    });

    // 3. Response arrives
    state.update(Event::TextStart {
        id: "req.0".to_string(),
    });
    state.update(Event::ResponseDelta {
        id: "req.0".to_string(),
        content: "say hello\n".to_string(),
    });
    state.update(Event::TextEnd {
        id: "req.0".to_string(),
    });

    // 4. Turn completes
    state.update(Event::TurnComplete {
        id: "req.0".to_string(),
        duration_secs: 0.5,
    });
    state.update(Event::Done {
        id: "req.0".to_string(),
    });

    // Build the feed
    let feed = LazyCache::feed(&state);
    let user_elements: Vec<_> = feed
        .elements
        .iter()
        .filter(|e| matches!(e, Element::UserMessage { .. }))
        .collect();

    // Count user messages in session
    let user_messages: Vec<_> = state
        .session()
        .messages
        .iter()
        .filter(|m| m.role == Role::User)
        .collect();

    println!(
        "Session messages: {:?}",
        state
            .session()
            .messages
            .iter()
            .map(|m| (m.role.clone(), m.content()))
            .collect::<Vec<_>>()
    );
    println!("User messages in session: {}", user_messages.len());
    println!(
        "Feed elements: {:?}",
        feed.elements
            .iter()
            .map(|e| format!("{:?}", e))
            .collect::<Vec<_>>()
    );
    println!("UserMessage elements in feed: {}", user_elements.len());

    assert_eq!(
        user_messages.len(),
        1,
        "Expected 1 user message in session, got {}: {:?}",
        user_messages.len(),
        user_messages
            .iter()
            .map(|m| m.content())
            .collect::<Vec<_>>()
    );

    assert_eq!(
        user_elements.len(),
        1,
        "Expected 1 UserMessage element in feed, got {}",
        user_elements.len()
    );
}

/// Test the full submit flow as it happens in the TUI.
#[test]
fn test_full_submit_flow() {
    let mut state = AppState::default();

    // Simulate typing and submitting
    state.update(Event::Input('s'));
    state.update(Event::Input('a'));
    state.update(Event::Input('y'));
    state.update(Event::Input(' '));
    state.update(Event::Input('h'));
    state.update(Event::Input('e'));
    state.update(Event::Input('l'));
    state.update(Event::Input('l'));
    state.update(Event::Input('o'));
    state.update(Event::submit());

    // After submit, check that message is in session
    let user_messages: Vec<_> = state
        .session()
        .messages
        .iter()
        .filter(|m| m.role == Role::User)
        .collect();

    println!(
        "After submit - user messages in session: {}",
        user_messages.len()
    );
    for (i, msg) in state.session().messages.iter().enumerate() {
        println!("  [{}] {:?}: {:?}", i, msg.role, msg.content());
    }

    assert_eq!(
        user_messages.len(),
        1,
        "Expected 1 user message after submit, got {}: {:?}",
        user_messages.len(),
        user_messages
            .iter()
            .map(|m| m.content())
            .collect::<Vec<_>>()
    );

    // Pop from request queue and simulate agent
    state.pop_queue();
    state.agent_state_mut().streaming = true;
    state.agent_state_mut().turn_active = true;

    // Response arrives (simulate mock echo)
    state.update(Event::Response {
        id: "req.0".to_string(),
        content: "say hello\n".to_string(),
        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),
    });

    // Turn completes
    state.update(Event::TurnComplete {
        id: "req.0".to_string(),
        duration_secs: 0.5,
    });
    state.update(Event::Done {
        id: "req.0".to_string(),
    });

    // Build the feed
    let feed = LazyCache::feed(&state);
    let user_elements: Vec<_> = feed
        .elements
        .iter()
        .filter(|e| matches!(e, Element::UserMessage { .. }))
        .collect();

    println!(
        "\nFeed elements: {:?}",
        feed.elements
            .iter()
            .map(|e| format!("{:?}", e))
            .collect::<Vec<_>>()
    );
    println!("UserMessage elements in feed: {}", user_elements.len());

    assert_eq!(
        user_elements.len(),
        1,
        "Expected 1 UserMessage element in feed, got {}",
        user_elements.len()
    );
}

/// Test that applying UserMessageSubmitted twice does NOT add two messages.
/// This is a regression test for potential double-submit issues.
#[test]
fn test_apply_user_message_twice_should_not_duplicate() {
    let mut state = AppState::default();

    // Apply the same message twice (simulating potential double-submit)
    state.update(Event::UserMessageSubmitted {
        id: "req.0".to_string(),
        content: "hello".to_string(),
    });

    state.update(Event::UserMessageSubmitted {
        id: "req.0".to_string(), // Same ID!
        content: "hello".to_string(),
    });

    let user_messages: Vec<_> = state
        .session()
        .messages
        .iter()
        .filter(|m| m.role == Role::User)
        .collect();

    println!(
        "After applying same message twice - user messages: {}",
        user_messages.len()
    );
    for (i, msg) in state.session().messages.iter().enumerate() {
        println!("  [{}] id={:?} content={:?}", i, msg.id, msg.content());
    }

    // With the idempotency fix, this should now show 1 message.
    assert_eq!(
        user_messages.len(),
        1,
        "Expected 1 user message (idempotent), got {}",
        user_messages.len()
    );
}

/// Test what happens when submit is called twice rapidly.
#[test]
fn test_rapid_duplicate_submit() {
    let mut state = AppState::default();

    // Simulate two rapid submissions
    state.update(Event::Input('a'));
    state.update(Event::submit());

    state.update(Event::Input('b'));
    state.update(Event::submit());

    let user_messages: Vec<_> = state
        .session()
        .messages
        .iter()
        .filter(|m| m.role == Role::User)
        .collect();

    println!("After two submits - user messages: {}", user_messages.len());
    for (i, msg) in state.session().messages.iter().enumerate() {
        println!("  [{}] id={:?} content={:?}", i, msg.id, msg.content());
    }

    // Two different messages should produce two entries
    assert_eq!(
        user_messages.len(),
        2,
        "Expected 2 different user messages, got {}",
        user_messages.len()
    );
}
