use crate::event::Event;
use crate::model::{AppState, Role};

fn fresh_state() -> AppState {
    AppState::default()
}

fn user_count(state: &AppState) -> usize {
    state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::User)
        .count()
}

#[test]
fn test_complete_agent_flow() {
    let mut state = fresh_state();
    state.update(Event::Input('H'));
    state.update(Event::Submit);
    assert_eq!(state.session.messages.len(), 1);
    assert_eq!(state.session.messages[0].role, Role::User);
    assert!(!state.agent.streaming);
    state.pop_queue();
    state.agent.streaming = true;
    state.update(Event::AgentThinking {
        id: "req.0".to_string(),
    });
    assert!(state.agent.streaming);
    state.update(Event::AgentThoughtDone {
        id: "req.0".to_string(),
    });
    state.update(Event::AgentResponse {
        id: "req.0".to_string(),
        content: "Hello".to_string(),
    });
    assert_eq!(state.session.messages.len(), 3);
    assert_eq!(state.session.messages[1].role, Role::Thought);
    assert_eq!(state.session.messages[2].role, Role::Assistant);
    state.update(Event::AgentDone {
        id: "req.0".to_string(),
    });
    assert!(!state.agent.streaming);
}

#[test]
fn test_queue_processing() {
    let mut state = fresh_state();
    state.update(Event::Input('A'));
    state.update(Event::Submit);
    state.update(Event::Input('B'));
    state.update(Event::Submit);
    assert_eq!(state.session.messages.len(), 2);
    assert_eq!(state.agent.request_queue.len(), 2);
    assert!(!state.agent.streaming);
}

#[test]
fn test_second_submit_while_turn_active_queues_message() {
    let mut state = fresh_state();
    // First message submitted and sent to agent
    state.update(Event::Input('A'));
    state.update(Event::Submit);
    assert_eq!(state.session.messages.len(), 1);
    assert_eq!(state.session.messages[0].content, "A");

    // Simulate what spawn_if_queued should do: set turn_active
    state.agent.turn_active = true;

    // Second message while turn is active
    state.update(Event::Input('B'));
    state.update(Event::Submit);

    // Message B should NOT appear in chat yet — queued for next turn
    assert_eq!(
        state.session.messages.len(),
        1,
        "Message B should not appear until its turn starts"
    );
    assert_eq!(
        state.agent.message_queue.len(),
        1,
        "Message B should be in message_queue"
    );
    assert_eq!(state.agent.message_queue[0].content, "B");
}

#[test]
fn test_queued_message_appears_after_turn_completes() {
    let mut state = fresh_state();
    // Submit first message and simulate agent start
    state.update(Event::Input('A'));
    state.update(Event::Submit);
    state.agent.turn_active = true;

    // Submit second message while agent is working
    state.update(Event::Input('B'));
    state.update(Event::Submit);
    assert_eq!(
        state.session.messages.len(),
        1,
        "Only message A visible during turn"
    );

    // Agent finishes turn
    state.update(Event::AgentDone {
        id: "req.0".to_string(),
    });

    // Now message B should appear and be ready for its turn
    assert!(
        state
            .session
            .messages
            .iter()
            .any(|m| m.role == Role::User && m.content == "B"),
        "Message B should appear in chat after previous turn completes"
    );
}

#[test]
fn test_three_messages_one_at_a_time() {
    let mut state = fresh_state();
    state.update(Event::Input('1'));
    state.update(Event::Submit);
    state.agent.turn_active = true;

    state.update(Event::Input('2'));
    state.update(Event::Submit);
    state.update(Event::Input('3'));
    state.update(Event::Submit);

    assert_eq!(
        user_count(&state),
        1,
        "Only first message visible during active turn"
    );
    assert_eq!(
        state.agent.message_queue.len(),
        2,
        "Messages 2 and 3 queued"
    );

    state.update(Event::AgentDone { id: "req.0".into() });
    assert_eq!(
        user_count(&state),
        2,
        "Message 2 appears after turn 1 completes"
    );
    assert_eq!(state.agent.message_queue.len(), 1, "Message 3 still queued");

    state.update(Event::AgentDone { id: "req.1".into() });
    assert_eq!(
        user_count(&state),
        3,
        "Message 3 appears after turn 2 completes"
    );
    assert!(
        state.agent.message_queue.is_empty(),
        "Queue empty after all delivered"
    );
}

#[test]
fn test_submit_adds_message_to_queue() {
    let mut state = fresh_state();
    state.update(Event::Input('H'));
    state.update(Event::Submit);
    assert_eq!(state.session.messages.len(), 1);
    assert_eq!(state.session.messages[0].role, Role::User);
    assert_eq!(state.agent.request_queue.len(), 1);
}

#[test]
fn test_messages_have_correlation_id() {
    let mut state = fresh_state();
    state.update(Event::Input('H'));
    state.update(Event::Submit);
    assert_eq!(state.session.messages.len(), 1);
    assert!(state.session.messages[0].id.starts_with("req."));
}

#[test]
fn test_multiple_submits_increment_id() {
    let mut state = fresh_state();
    state.update(Event::Input('A'));
    state.update(Event::Submit);
    let first_id = state.session.messages[0].id.clone();
    state.update(Event::Input('B'));
    state.update(Event::Submit);
    let second_id = state.session.messages[1].id.clone();
    assert_ne!(first_id, second_id);
}

#[test]
fn test_multiple_thoughts_for_sequential_requests() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.update(Event::AgentThinking {
        id: "req.0".to_string(),
    });
    state.update(Event::AgentThoughtDone {
        id: "req.0".to_string(),
    });
    state.update(Event::AgentResponse {
        id: "req.0".to_string(),
        content: "A".to_string(),
    });
    state.update(Event::AgentDone {
        id: "req.0".to_string(),
    });
    state.update(Event::AgentThinking {
        id: "req.1".to_string(),
    });
    state.update(Event::AgentThoughtDone {
        id: "req.1".to_string(),
    });
    state.update(Event::AgentResponse {
        id: "req.1".to_string(),
        content: "B".to_string(),
    });
    let thoughts: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::Thought)
        .collect();
    assert_eq!(thoughts.len(), 2);
}
