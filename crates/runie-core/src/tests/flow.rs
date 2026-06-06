use crate::model::AppState;
use crate::event::Event;
use crate::ui::format_messages;

fn fresh_state() -> AppState {
    AppState::default()
}

#[test]
fn test_complete_agent_flow() {
    let mut state = fresh_state();
    state.update(Event::Input('H'));
    state.update(Event::Submit);

    assert_eq!(state.messages.len(), 1);
    assert_eq!(state.messages[0].role, "user");
    assert!(!state.streaming);

    state.pop_queue();
    state.streaming = true;

    state.update(Event::AgentThinking { id: "req.0".to_string() });
    assert!(state.streaming);

    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
    state.update(Event::AgentResponse {
        id: "req.0".to_string(),
        content: "Hello".to_string()
    });

    assert_eq!(state.messages.len(), 3);
    assert_eq!(state.messages[1].role, "thought");
    assert_eq!(state.messages[2].role, "assistant");

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
    assert_eq!(state.messages[0].role, "user");
    assert_eq!(state.request_queue.len(), 1);
}

#[test]
fn test_messages_have_correlation_id() {
    let mut state = fresh_state();
    state.update(Event::Input('H'));
    state.update(Event::Submit);

    assert_eq!(state.messages.len(), 1);
    assert!(state.messages[0].id.starts_with("req."));
}

#[test]
fn test_multiple_submits_increment_id() {
    let mut state = fresh_state();

    state.update(Event::Input('A'));
    state.update(Event::Submit);
    let first_id = state.messages[0].id.clone();

    state.update(Event::Input('B'));
    state.update(Event::Submit);
    let second_id = state.messages[1].id.clone();

    assert_ne!(first_id, second_id);
}
