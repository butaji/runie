use crate::model::{AppState, Role};
use crate::tests::fresh_state;
use crate::Event;

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
    state.update(crate::Event::Input('H'));
    state.update(Event::submit());
    assert_eq!(state.session.messages.len(), 1);
    assert_eq!(state.session.messages[0].role, Role::User);
    assert!(!state.agent.streaming);
    state.pop_queue();
    state.agent.streaming = true;
    state.update(crate::Event::Thinking {
        id: "req.0".to_string(),
    });
    assert!(state.agent.streaming);
    state.update(crate::Event::ThoughtDone {
        id: "req.0".to_string(),
    });
    state.update(crate::Event::Response {
        id: "req.0".to_string(),
        content: "Hello".to_string(),
    });
    assert_eq!(state.session.messages.len(), 3);
    assert_eq!(state.session.messages[1].role, Role::Thought);
    assert_eq!(state.session.messages[2].role, Role::Assistant);
    state.update(crate::Event::Done {
        id: "req.0".to_string(),
    });
    assert!(!state.agent.streaming);
}

#[test]
fn test_queue_processing() {
    let mut state = fresh_state();
    state.update(crate::Event::Input('A'));
    state.update(Event::submit());
    state.update(crate::Event::Input('B'));
    state.update(Event::submit());
    assert_eq!(state.session.messages.len(), 2);
    assert_eq!(state.agent.request_queue.len(), 2);
    assert!(!state.agent.streaming);
}

#[test]
fn test_second_submit_while_turn_active_queues_message() {
    let mut state = fresh_state();
    // First message submitted and sent to agent
    state.update(crate::Event::Input('A'));
    state.update(Event::submit());
    assert_eq!(state.session.messages.len(), 1);
    assert_eq!(state.session.messages[0].content(), "A");

    // Simulate what spawn_if_queued should do: set turn_active
    state.agent.turn_active = true;

    // Second message while turn is active
    state.update(crate::Event::Input('B'));
    state.update(Event::submit());

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
    state.update(crate::Event::Input('A'));
    state.update(Event::submit());
    state.agent.turn_active = true;

    // Submit second message while agent is working
    state.update(crate::Event::Input('B'));
    state.update(Event::submit());
    assert_eq!(
        state.session.messages.len(),
        1,
        "Only message A visible during turn"
    );

    // Agent finishes turn
    state.update(crate::Event::Done {
        id: "req.0".to_string(),
    });

    // Now message B should appear and be ready for its turn
    assert!(
        state
            .session
            .messages
            .iter()
            .any(|m| m.role == Role::User && m.content() == "B"),
        "Message B should appear in chat after previous turn completes"
    );
}

fn submit_queued_message(state: &mut AppState, ch: char) {
    state.update(crate::Event::Input(ch));
    state.update(Event::submit());
}

#[test]
fn test_three_messages_one_at_a_time() {
    let mut state = fresh_state();
    submit_queued_message(&mut state, '1');
    state.agent.turn_active = true;

    submit_queued_message(&mut state, '2');
    submit_queued_message(&mut state, '3');

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

    state.update(crate::Event::Done { id: "req.0".into() });
    assert_eq!(
        user_count(&state),
        2,
        "Message 2 appears after turn 1 completes"
    );
    assert_eq!(state.agent.message_queue.len(), 1, "Message 3 still queued");

    state.update(crate::Event::Done { id: "req.1".into() });
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
    state.update(crate::Event::Input('H'));
    state.update(Event::submit());
    assert_eq!(state.session.messages.len(), 1);
    assert_eq!(state.session.messages[0].role, Role::User);
    assert_eq!(state.agent.request_queue.len(), 1);
}

#[test]
fn test_messages_have_correlation_id() {
    let mut state = fresh_state();
    state.update(crate::Event::Input('H'));
    state.update(Event::submit());
    assert_eq!(state.session.messages.len(), 1);
    assert!(state.session.messages[0].id.starts_with("req."));
}

#[test]
fn test_multiple_submits_increment_id() {
    let mut state = fresh_state();
    state.update(crate::Event::Input('A'));
    state.update(Event::submit());
    let first_id = state.session.messages[0].id.clone();
    state.update(crate::Event::Input('B'));
    state.update(Event::submit());
    let second_id = state.session.messages[1].id.clone();
    assert_ne!(first_id, second_id);
}

#[test]
fn test_multiple_thoughts_for_sequential_requests() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.update(crate::Event::Thinking {
        id: "req.0".to_string(),
    });
    state.update(crate::Event::ThoughtDone {
        id: "req.0".to_string(),
    });
    state.update(crate::Event::Response {
        id: "req.0".to_string(),
        content: "A".to_string(),
    });
    state.update(crate::Event::Done {
        id: "req.0".to_string(),
    });
    state.update(crate::Event::Thinking {
        id: "req.1".to_string(),
    });
    state.update(crate::Event::ThoughtDone {
        id: "req.1".to_string(),
    });
    state.update(crate::Event::Response {
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

#[test]
fn test_think_tags_split_into_thought_and_answer() {
    let mut state = fresh_state();
    state.update(crate::Event::Thinking {
        id: "req.0".to_string(),
    });
    state.update(crate::Event::Response {
        id: "req.0".to_string(),
        content: "<think>reasoning</think>answer".to_string(),
    });
    state.update(crate::Event::ThoughtDone {
        id: "req.0".to_string(),
    });
    let thoughts: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::Thought)
        .collect();
    assert_eq!(thoughts.len(), 1);
    assert!(thoughts[0].content().contains("reasoning"));
    let assistants: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::Assistant)
        .collect();
    assert_eq!(assistants.len(), 1);
    assert_eq!(assistants[0].content(), "answer");
    // Cached assistant index must point to the answer, not the thought.
    assert_eq!(state.agent.last_assistant_index, Some(1));
}

#[test]
fn test_think_tags_only_reasoning_removes_assistant() {
    let mut state = fresh_state();
    state.update(crate::Event::Thinking {
        id: "req.0".to_string(),
    });
    state.update(crate::Event::Response {
        id: "req.0".to_string(),
        content: "<think>only reasoning</think>".to_string(),
    });
    state.update(crate::Event::ThoughtDone {
        id: "req.0".to_string(),
    });
    let assistants: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::Assistant)
        .collect();
    assert!(assistants.is_empty());
    assert!(state.agent.last_assistant_index.is_none());
}

#[test]
fn test_unclosed_think_tag_hides_reasoning() {
    let mut state = fresh_state();
    state.update(crate::Event::Thinking {
        id: "req.0".to_string(),
    });
    state.update(crate::Event::Response {
        id: "req.0".to_string(),
        content: "visible<think>still reasoning".to_string(),
    });
    state.update(crate::Event::ThoughtDone {
        id: "req.0".to_string(),
    });
    let assistants: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::Assistant)
        .collect();
    assert_eq!(assistants.len(), 1);
    assert_eq!(assistants[0].content(), "visible");
    let thoughts: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::Thought)
        .collect();
    assert_eq!(thoughts.len(), 1);
    assert!(thoughts[0].content().contains("still reasoning"));
}

#[test]
fn test_think_tags_update_cached_assistant_index_for_tail_flush() {
    let mut state = fresh_state();
    state.update(crate::Event::Thinking {
        id: "req.0".to_string(),
    });
    state.update(crate::Event::Response {
        id: "req.0".to_string(),
        content: "<think>reasoning</think>answer".to_string(),
    });
    state.update(crate::Event::ThoughtDone {
        id: "req.0".to_string(),
    });
    assert_eq!(state.agent.last_assistant_index, Some(1));

    // Simulate a trailing streaming delta being flushed on turn end.
    state.agent.streaming_buffer.push_delta(" tail");
    let tail = state.agent.streaming_buffer.force_flush().join("");
    if !tail.is_empty() {
        if let Some(idx) = state.agent.last_assistant_index {
            state.session.messages[idx].push_text_part(&tail);
        }
    }
    let assistants: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::Assistant)
        .collect();
    assert_eq!(assistants[0].content(), "answer tail");
    let thoughts: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::Thought)
        .collect();
    assert!(
        !thoughts[0].content().contains("tail"),
        "tail must not leak into thought"
    );
}
