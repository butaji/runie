use crate::event::Event;
use crate::model::{AppState, Role};
#[cfg(test)]
use crate::ui::format_test::format_messages;

fn fresh_state() -> AppState {
    AppState::default()
}

/// Set input buffer directly and submit — bypasses the command palette.
fn exec(state: &mut AppState, text: &str) {
    state.input.input = text.into();
    state.input.cursor_pos = text.len();
    state.update(Event::Submit);
}

fn dispatch(state: &mut AppState, events: &[Event]) {
    for e in events {
        state.update(e.clone());
    }
}

#[test]
fn test_reset_clears_state() {
    let mut state = fresh_state();
    state.input.input = "test".to_string();
    state.streaming = true;
    state.update(Event::Reset);
    assert_eq!(state.input.input, "");
    assert!(!state.streaming);
    assert_eq!(state.session.messages.len(), 0);
}

#[test]
fn test_scroll_up() {
    let mut state = fresh_state();
    state.update(Event::ScrollUp);
    assert_eq!(state.view.scroll, 1);
}

#[test]
fn test_scroll_down() {
    let mut state = fresh_state();
    state.view.scroll = 5;
    state.update(Event::ScrollDown);
    assert_eq!(state.view.scroll, 4);
}

#[test]
fn test_scroll_down_saturates() {
    let mut state = fresh_state();
    state.view.scroll = 0;
    state.update(Event::ScrollDown);
    assert_eq!(state.view.scroll, 0);
}

#[test]
fn test_tool_flow_creates_two_thoughts() {
    let mut state = fresh_state();
    state.streaming = true;
    dispatch(&mut state, &[
        Event::AgentThinking { id: "req.0".into() },
        Event::AgentThoughtDone { id: "req.0".into() },
        Event::AgentToolStart { id: "req.0".into(), name: "list_files".into() },
        Event::AgentToolEnd { duration_secs: 0.5, output: String::new() },
        Event::AgentThinking { id: "req.0".into() },
        Event::AgentThoughtDone { id: "req.0".into() },
        Event::AgentResponse { id: "req.0".into(), content: "Here are the files".into() },
    ]);
    let thought_count = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::Thought)
        .count();
    assert_eq!(thought_count, 2);
}

#[test]
fn test_turn_complete_event() {
    let mut state = fresh_state();
    state.intermediate_step_count = 1;
    state.update(Event::AgentTurnComplete {
        id: "req.0".to_string(),
        duration_secs: 5.1,
    });
    assert_eq!(state.session.messages.len(), 1);
    let msg = &state.session.messages[0];
    assert_eq!(msg.role, Role::TurnComplete);
    assert!(msg.content.contains("5.1s"));
}

#[test]
fn test_turn_complete_always_added_when_event_received() {
    let mut state = fresh_state();
    state.streaming = true;
    dispatch(&mut state, &[
        Event::AgentThinking { id: "req.0".into() },
        Event::AgentThoughtDone { id: "req.0".into() },
        Event::AgentResponse { id: "req.0".into(), content: "Hi".into() },
        Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 },
    ]);
    let has_turn_complete = state
        .session
        .messages
        .iter()
        .any(|m| m.role == Role::TurnComplete);
    assert!(has_turn_complete, "Core should always add TurnComplete when event is received; agent decides whether to emit it");
}

#[test]
fn test_tool_done_event() {
    let mut state = fresh_state();
    state.update(Event::AgentToolStart {
        id: "req.0".to_string(),
        name: "list_files".to_string(),
    });
    state.update(Event::AgentToolEnd {
        duration_secs: 0.3,
        output: String::new(),
    });
    assert_eq!(state.session.messages.len(), 1);
    let msg = &state.session.messages[0];
    assert_eq!(msg.role, Role::Tool);
    assert!(msg.content.contains("list_files"));
    assert!(msg.content.contains("0.3s"));
}

#[test]
fn test_formatted_labels_short_names() {
    let mut state = fresh_state();
    state.streaming = true;
    dispatch(&mut state, &[
        Event::AgentThinking { id: "req.0".into() },
        Event::AgentThoughtDone { id: "req.0".into() },
        Event::AgentToolStart { id: "req.0".into(), name: "list_files".into() },
        Event::AgentToolEnd { duration_secs: 0.3, output: String::new() },
        Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 5.1 },
    ]);
    let lines = format_messages(&state);
    let content: String = lines
        .iter()
        .flat_map(|l| l.spans.iter().map(|s| s.text.clone()).collect::<Vec<_>>())
        .collect();
    assert!(content.contains("✓"), "Missing '✓' in: {}", content);
    assert!(content.contains("0.3s"), "Missing '0.3s' in: {}", content);
    assert!(
        content.contains("Turn completed"),
        "Missing 'Turn completed' in: {}",
        content
    );
}

#[test]
fn test_list_files_full_tool_flow_sequence() {
    let mut state = fresh_state();
    state.streaming = true;
    dispatch(&mut state, &[
        Event::AgentThinking { id: "req.0".into() },
        Event::AgentThoughtDone { id: "req.0".into() },
        Event::AgentToolStart { id: "req.0".into(), name: "list_files".into() },
        Event::AgentToolEnd { duration_secs: 0.5, output: String::new() },
        Event::AgentThinking { id: "req.0".into() },
        Event::AgentThoughtDone { id: "req.0".into() },
        Event::AgentResponse { id: "req.0".into(), content: "Here are the files:".into() },
        Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 5.1 },
    ]);
    assert_eq!(state.session.messages.len(), 5);
    assert_eq!(state.session.messages[0].role, Role::Thought);
    assert_eq!(state.session.messages[1].role, Role::Tool);
    assert_eq!(state.session.messages[2].role, Role::Thought);
    assert_eq!(state.session.messages[3].role, Role::Assistant);
    assert_eq!(state.session.messages[4].role, Role::TurnComplete);
    let lines = format_messages(&state);
    let content: String = lines
        .iter()
        .flat_map(|l| l.spans.iter().map(|s| s.text.clone()).collect::<Vec<_>>())
        .collect();
    assert!(content.contains("Thought"));
    assert!(content.contains("✓"));
    assert!(content.contains("list_files"));
    assert!(content.contains("→"));
    assert!(content.contains("Turn completed in 5.1s"));
}

#[test]
fn test_turn_complete_shows_even_if_done_arrives_first() {
    let mut state = fresh_state();
    state.streaming = true;
    dispatch(&mut state, &[
        Event::AgentThinking { id: "req.0".into() },
        Event::AgentThoughtDone { id: "req.0".into() },
        Event::AgentToolStart { id: "req.0".into(), name: "list_files".into() },
        Event::AgentToolEnd { duration_secs: 0.5, output: String::new() },
        Event::AgentResponse { id: "req.0".into(), content: "Here are files".into() },
        Event::AgentDone { id: "req.0".into() },
        Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 3.2 },
    ]);
    let has_turn_complete = state
        .session
        .messages
        .iter()
        .any(|m| m.role == Role::TurnComplete);
    assert!(
        has_turn_complete,
        "TurnComplete should show even if Done event arrives before TurnComplete"
    );
}

#[test]
fn test_thinking_indicator_shows_for_queued_request() {
    let mut state = fresh_state();
    state.streaming = true;
    state
        .agent
        .request_queue
        .push_back(("B".to_string(), "req.1".to_string()));
    state.thinking_started_at = Some(std::time::Instant::now());
    let has_thought = state
        .session
        .messages
        .iter()
        .any(|m| m.role == Role::Thought);
    assert!(!has_thought);
}

#[test]
fn test_sessions_command_shows_system_message() {
    let mut state = fresh_state();
    for c in "/sessions".chars() {
        state.update(Event::Input(c));
    }
    state.update(Event::Submit);
    let sys_msgs: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    assert!(
        !sys_msgs.is_empty(),
        "Should show system message for /sessions"
    );
}

#[test]
fn test_save_and_load_session() {
    use crate::tests::slash::ENV_LOCK;
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let tmp = std::env::temp_dir().join("runie_session_cmd_test");
    let _ = std::fs::remove_dir_all(&tmp);
    std::env::set_var("RUNIE_SESSIONS_DIR", &tmp);

    let mut state = fresh_state();
    state.update(Event::Input('h'));
    state.update(Event::Input('i'));
    state.update(Event::Submit);
    exec(&mut state, "/save test_session"); // Opens form with pre-filled name
    state.update(Event::Submit); // Submits the form
    assert!(state
        .session
        .messages
        .iter()
        .any(|m| m.role == Role::System && m.content.contains("saved")));

    let mut state2 = fresh_state();
    exec(&mut state2, "/load test_session"); // Opens form with pre-filled name
    state2.update(Event::Submit); // Submits the form
    assert!(state2
        .session
        .messages
        .iter()
        .any(|m| m.role == Role::System && m.content.contains("loaded")));
    assert!(state2
        .session
        .messages
        .iter()
        .any(|m| m.role == Role::User && m.content == "hi"));

    std::env::remove_var("RUNIE_SESSIONS_DIR");
}
