use crate::model::{AppState, Role};
use crate::event::Event;

fn fresh_state() -> AppState {
    AppState::default()
}

#[test]
fn test_complete_agent_flow() {
    let mut state = fresh_state();
    state.update(Event::Input('H'));
    state.update(Event::Submit);
    assert_eq!(state.messages.len(), 1);
    assert_eq!(state.messages[0].role, Role::User);
    assert!(!state.streaming);
    state.pop_queue();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    assert!(state.streaming);
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "Hello".to_string() });
    assert_eq!(state.messages.len(), 3);
    assert_eq!(state.messages[1].role, Role::Thought);
    assert_eq!(state.messages[2].role, Role::Assistant);
    state.update(Event::AgentDone { id: "req.0".to_string() });
    assert!(!state.streaming);
}

#[test]
fn test_queue_processing() {
    let mut state = fresh_state();
    state.update(Event::Input('A'));
    state.update(Event::Submit);
    state.update(Event::Input('B'));
    state.update(Event::Submit);
    assert_eq!(state.messages.len(), 2);
    assert_eq!(state.request_queue.len(), 2);
    assert!(!state.streaming);
}

#[test]
fn test_submit_adds_message_to_queue() {
    let mut state = fresh_state();
    state.update(Event::Input('H'));
    state.update(Event::Submit);
    assert_eq!(state.messages.len(), 1);
    assert_eq!(state.messages[0].role, Role::User);
    assert_eq!(state.request_queue.len(), 1);
}

#[test]
fn test_multiple_thoughts_for_sequential_requests() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "A".to_string() });
    state.update(Event::AgentDone { id: "req.0".to_string() });
    state.update(Event::AgentThinking { id: "req.1".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.1".to_string() });
    state.update(Event::AgentResponse { id: "req.1".to_string(), content: "B".to_string() });
    let thoughts: Vec<_> = state.messages.iter().filter(|m| m.role == Role::Thought).collect();
    assert_eq!(thoughts.len(), 2);
}

#[test]
fn test_inflight_counter_prevents_premature_streaming_stop() {
    // BUG: When user submits A then B, B is popped from queue immediately
    // and sent to agent. When A's AgentDone arrives, queue appears empty
    // so streaming was incorrectly set to false.
    // FIX: inflight counter tracks commands in agent, not queue length.
    let mut state = fresh_state();

    // Simulate: submit A, pop and send to agent (inflight=1)
    state.update(Event::Input('A'));
    state.update(Event::Submit);
    state.pop_queue();
    state.inflight = 1;
    state.streaming = true;

    // Simulate: submit B, pop and send to agent (inflight=2)
    state.update(Event::Input('B'));
    state.update(Event::Submit);
    state.pop_queue();
    state.inflight = 2;

    // Queue is now empty but 2 commands are in-flight
    assert!(state.request_queue.is_empty());
    assert_eq!(state.inflight, 2);

    // A completes
    state.update(Event::AgentDone { id: "req.0".to_string() });
    // streaming should STILL be true because B is in-flight
    assert!(state.streaming, "streaming should stay true while B is in-flight");
    assert_eq!(state.inflight, 1);

    // B completes
    state.update(Event::AgentDone { id: "req.1".to_string() });
    // Now streaming should stop
    assert!(!state.streaming, "streaming should stop when all done");
    assert_eq!(state.inflight, 0);
}
