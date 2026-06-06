use crate::model::AppState;
use crate::event::Event;
use crate::update::update;

fn fresh_state() -> AppState {
    AppState::default()
}

#[test]
fn test_complete_agent_flow() {
    let mut state = fresh_state();
    state = update(state, Event::Input('H'));
    state = update(state, Event::Submit);
    assert_eq!(state.messages.len(), 1);
    assert_eq!(state.messages[0].role, "user");
    assert!(!state.streaming);
    state.pop_queue();
    state.streaming = true;
    let state = update(state, Event::AgentThinking { id: "req.0".to_string() });
    assert!(state.streaming);
    let state = update(state, Event::AgentThoughtDone { id: "req.0".to_string() });
    let state = update(state, Event::AgentResponse { 
        id: "req.0".to_string(),
        content: "Hello".to_string() 
    });
    assert_eq!(state.messages.len(), 3);
    assert_eq!(state.messages[1].role, "thought");
    assert_eq!(state.messages[2].role, "assistant");
    let state = update(state, Event::AgentDone { id: "req.0".to_string() });
    assert!(!state.streaming);
}

#[test]
fn test_queue_processing() {
    let state = fresh_state();
    let state = update(state, Event::Input('A'));
    let state = update(state, Event::Submit);
    let state = update(state, Event::Input('B'));
    let state = update(state, Event::Submit);
    assert_eq!(state.messages.len(), 2);
    assert_eq!(state.request_queue.len(), 2);
    assert!(!state.streaming);
}

#[test]
fn test_submit_adds_message_to_queue() {
    let state = update(update(fresh_state(), Event::Input('H')), Event::Submit);
    assert_eq!(state.messages.len(), 1);
    assert_eq!(state.messages[0].role, "user");
    assert_eq!(state.request_queue.len(), 1);
}

#[test]
fn test_multiple_thoughts_for_sequential_requests() {
    let mut state = fresh_state();
    state.streaming = true;
    state = update(state, Event::AgentThinking { id: "req.0".to_string() });
    state = update(state, Event::AgentThoughtDone { id: "req.0".to_string() });
    state = update(state, Event::AgentResponse { id: "req.0".to_string(), content: "A".to_string() });
    state = update(state, Event::AgentDone { id: "req.0".to_string() });
    state = update(state, Event::AgentThinking { id: "req.1".to_string() });
    state = update(state, Event::AgentThoughtDone { id: "req.1".to_string() });
    state = update(state, Event::AgentResponse { id: "req.1".to_string(), content: "B".to_string() });
    let thoughts: Vec<_> = state.messages.iter().filter(|m| m.role == "thought").collect();
    assert_eq!(thoughts.len(), 2);
}
