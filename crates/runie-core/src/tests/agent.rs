use crate::model::AppState;
use crate::event::Event;

fn fresh_state() -> AppState {
    AppState::default()
}

#[test]
fn test_agent_thinking_sets_streaming() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".to_string() });

    assert!(state.streaming);
    assert!(state.thinking_started_at.is_some());
}

#[test]
fn test_agent_response_creates_message() {
    let mut state = fresh_state();
    state.streaming = true;

    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
    state.update(Event::AgentResponse {
        id: "req.0".to_string(),
        content: "Hello".to_string()
    });

    assert_eq!(state.messages.len(), 2);
    assert_eq!(state.messages[1].role, "assistant");
    assert_eq!(state.messages[1].content, "Hello");
}

#[test]
fn test_agent_response_appends_to_existing() {
    let mut state = fresh_state();
    state.streaming = true;

    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
    state.update(Event::AgentResponse {
        id: "req.0".to_string(),
        content: "Hello ".to_string()
    });
    state.update(Event::AgentResponse {
        id: "req.0".to_string(),
        content: "World".to_string()
    });

    assert_eq!(state.messages.len(), 2);
    assert_eq!(state.messages[0].role, "thought");
    assert_eq!(state.messages[1].role, "assistant");
    assert_eq!(state.messages[1].content, "Hello World");
}

#[test]
fn test_agent_done_clears_streaming() {
    let mut state = fresh_state();
    state.streaming = true;
    state.thinking_started_at = Some(std::time::Instant::now());

    state.update(Event::AgentDone { id: "req.0".to_string() });

    assert!(!state.streaming);
    assert!(state.thinking_started_at.is_none());
}

#[test]
fn test_agent_error_creates_error_message() {
    let mut state = fresh_state();
    state.streaming = true;

    state.update(Event::AgentError {
        id: "req.0".to_string(),
        message: "Something went wrong".to_string()
    });

    assert!(!state.streaming);
    assert_eq!(state.messages.len(), 1);
    assert_eq!(state.messages[0].role, "assistant");
    assert!(state.messages[0].content.contains("Error"));
}
