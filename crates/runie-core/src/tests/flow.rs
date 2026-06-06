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
