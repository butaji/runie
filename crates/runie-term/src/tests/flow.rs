use super::*;



#[test]
fn test_submit_adds_message_to_queue() {
    let mut state = AppState::default();
    state.update(Event::Input('H'));
    state.update(Event::Input('i'));
    state.update(Event::Submit);
    assert_eq!(state.input.input, "");
    assert_eq!(state.session.messages.len(), 1);
    assert_eq!(state.session.messages[0].role, Role::User);
    assert_eq!(state.agent.request_queue.len(), 1);
}

#[test]
fn test_agent_thinking_sets_streaming() {
    let mut state = AppState::default();
    state.update(Event::Submit);
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    assert!(state.streaming);
    assert!(state.thinking_started_at.is_some());
}

#[test]
fn test_agent_response_creates_messages() {
    let mut state = AppState::default();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "Hello".to_string() });
    assert_eq!(state.session.messages.len(), 2);
    assert_eq!(state.session.messages[0].role, Role::Thought);
    assert_eq!(state.session.messages[1].role, Role::Assistant);
}

#[test]
fn test_agent_done_clears_streaming() {
    let mut state = AppState::default();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "Hi".to_string() });
    state.update(Event::AgentDone { id: "req.0".to_string() });
    assert!(!state.streaming);
}

#[test]
fn test_sequential_fifo_a_then_b() {
    let mut state = AppState::default();
    state.update(Event::Input('A'));
    state.update(Event::Submit);
    state.pop_queue();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "A".to_string() });
    state.update(Event::AgentDone { id: "req.0".to_string() });
    state.update(Event::AgentThinking { id: "req.1".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.1".to_string() });
    state.update(Event::AgentResponse { id: "req.1".to_string(), content: "B".to_string() });
    let thoughts: Vec<_> = state.session.messages.iter().filter(|m| m.role == Role::Thought).collect();
    assert_eq!(thoughts.len(), 2);
}



#[test]
fn test_full_list_files_integration() {
    let mut state = AppState::default();
    state.streaming = true;
    simulate_list_files_flow(&mut state);

    assert!(state.session.messages.iter().any(|m| m.role == Role::Thought));
    assert!(state.session.messages.iter().any(|m| m.role == Role::Tool));
    assert!(!state.streaming, "Streaming should stop after Done");
}

#[test]
fn test_list_files_command_flow() {
    let mut state = AppState::default();

    for c in "list files".chars() {
        state.update(Event::Input(c));
    }
    assert_eq!(state.input.input, "list files");

    state.update(Event::Submit);
    assert!(state.input.input.is_empty(), "Input cleared after submit");

    let (content, id) = state.peek_queue().expect("queued request");
    assert_eq!(content, "list files");
    assert!(id.starts_with("req."), "valid id");

    let (content, _id) = state.pop_queue().expect("pop");
    assert_eq!(content, "list files");
}

#[test]
fn test_list_files_message_content() {
    let mut state = AppState::default();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
    state.update(Event::AgentToolStart { id: "req.0".to_string(), name: "list_files".to_string() });
    state.update(Event::AgentToolEnd { duration_secs: 1.0, output: String::new() });
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "\nsrc/main.rs".to_string() });

    let assistant = state.session.messages.iter().find(|m| m.role == Role::Assistant).expect("assistant msg");
    assert!(assistant.content.contains("src/main.rs"), "Should contain file list");
}

#[test]
fn test_list_files_full_sequence() {
    let mut state = AppState::default();
    state.streaming = true;
    simulate_list_files_flow(&mut state);

    let msg = state.session.messages.iter().find(|m| m.role == Role::Assistant).expect("assistant msg");
    assert!(msg.content.contains("main.rs"));
    assert_eq!(state.session.messages.len(), 5, "expected 5 messages");
}
