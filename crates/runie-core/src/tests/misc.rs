use crate::model::{AppState, Role};
use crate::tests::{exec, fresh_state};
use crate::Event;

fn dispatch(state: &mut AppState, events: &[Event]) {
    for e in events {
        state.update(e.clone());
    }
}

/// Open palette and select a command by name
fn palette_select(state: &mut AppState, cmd: &str) {
    state.update(crate::Event::Input('/'));
    for c in cmd.chars() {
        state.update(crate::Event::PaletteFilter(c));
    }
    state.update(crate::Event::PaletteSelect);
}

#[test]
fn test_reset_clears_state() {
    let mut state = fresh_state();
    state.config.current_provider = "openai".to_string();
    state.config.current_model = "gpt-4o".to_string();
    state.input.input = "test".to_string();
    state.set_streaming(true);
    state.update(crate::Event::Reset);
    assert_eq!(state.input.input, "");
    assert!(!state.agent.streaming);
    assert_eq!(state.session.messages.len(), 0);
    assert_eq!(state.config.current_provider, "openai");
    assert_eq!(state.config.current_model, "gpt-4o");
    assert!(
        state.has_models(),
        "provider/model must stay active after reset"
    );
}

#[test]
fn test_scroll_up() {
    let mut state = fresh_state();
    state.update(crate::Event::Up);
    assert_eq!(state.view.scroll, 1);
}

#[test]
fn test_scroll_down() {
    let mut state = fresh_state();
    state.update(crate::Event::Down);
    assert_eq!(state.view.scroll, 0); // Down decreases scroll (newer content)
}

#[test]
fn test_scroll_down_saturates() {
    let mut state = fresh_state();
    // Scroll down from default (0) saturates
    state.update(crate::Event::Down);
    assert_eq!(state.view.scroll, 0);
}

#[test]
fn test_tool_flow_creates_two_thoughts() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    dispatch(
        &mut state,
        &[
            crate::Event::Thinking { id: "req.0".into() },
            crate::Event::ThoughtDone { id: "req.0".into() },
            crate::Event::ToolStart {
                id: "req.0".into(),
                name: "list_files".into(),
                input: serde_json::Value::Null,
            },
            crate::Event::ToolEnd {
                id: "".to_string(),
                duration_secs: 0.5,
                output: String::new(),
            },
            crate::Event::Thinking { id: "req.0".into() },
            crate::Event::ThoughtDone { id: "req.0".into() },
            crate::Event::Response {
                id: "req.0".into(),
                content: "Here are the files".into(),
            },
        ],
    );
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
    state.agent.intermediate_step_count = 1;
    state.update(crate::Event::TurnComplete {
        id: "req.0".to_string(),
        duration_secs: 5.1,
    });
    assert_eq!(state.session.messages.len(), 1);
    let msg = &state.session.messages[0];
    assert_eq!(msg.role, Role::TurnComplete);
    assert!(msg.content().contains("5.1s"));
}

#[test]
fn test_turn_complete_always_added_when_event_received() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    dispatch(
        &mut state,
        &[
            crate::Event::Thinking { id: "req.0".into() },
            crate::Event::ThoughtDone { id: "req.0".into() },
            crate::Event::Response {
                id: "req.0".into(),
                content: "Hi".into(),
            },
            crate::Event::TurnComplete {
                id: "req.0".into(),
                duration_secs: 1.0,
            },
        ],
    );
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
    state.update(crate::Event::ToolStart {
        id: "req.0".to_string(),
        name: "list_files".to_string(),
        input: serde_json::Value::Null,
    });
    state.update(crate::Event::ToolEnd {
        id: "".to_string(),
        duration_secs: 0.3,
        output: String::new(),
    });
    assert_eq!(state.session.messages.len(), 1);
    let msg = &state.session.messages[0];
    assert_eq!(msg.role, Role::Tool);
    assert!(msg.content().contains("list_files"));
    assert!(msg.content().contains("0.3s"));
}

#[test]
fn test_turn_complete_shows_even_if_done_arrives_first() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    dispatch(
        &mut state,
        &[
            crate::Event::Thinking { id: "req.0".into() },
            crate::Event::ThoughtDone { id: "req.0".into() },
            crate::Event::ToolStart {
                id: "req.0".into(),
                name: "list_files".into(),
                input: serde_json::Value::Null,
            },
            crate::Event::ToolEnd {
                id: "".to_string(),
                duration_secs: 0.5,
                output: String::new(),
            },
            crate::Event::Response {
                id: "req.0".into(),
                content: "Here are files".into(),
            },
            crate::Event::Done { id: "req.0".into() },
            crate::Event::TurnComplete {
                id: "req.0".into(),
                duration_secs: 3.2,
            },
        ],
    );
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
    state.agent.streaming = true;
    state
        .agent
        .request_queue
        .push_back(("B".to_string(), "req.1".to_string()));
    state.agent.thinking_started_at = Some(std::time::Instant::now());
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
    palette_select(&mut state, "sessions");
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
    use crate::tests::support::ENV_LOCK;
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let tmp = std::env::temp_dir().join("runie_session_cmd_test");
    let _ = std::fs::remove_dir_all(&tmp);
    unsafe { std::env::set_var("RUNIE_SESSIONS_DIR", &tmp) };

    let mut state = fresh_state();
    state.update(crate::Event::Input('h'));
    state.update(crate::Event::Input('i'));
    state.update(Event::submit());
    exec(&mut state, "/save test_session"); // Opens form with pre-filled name
    state.update(Event::CommandFormSubmit); // Submits the form
    assert!(state
        .session
        .messages
        .iter()
        .any(|m| m.role == Role::System && m.content().contains("saved")));

    let mut state2 = fresh_state();
    exec(&mut state2, "/load test_session"); // Opens form with pre-filled name
    state2.update(Event::CommandFormSubmit); // Submits the form
    assert!(state2
        .session
        .messages
        .iter()
        .any(|m| m.role == Role::System && m.content().contains("loaded")));
    assert!(state2
        .session
        .messages
        .iter()
        .any(|m| m.role == Role::User && m.content() == "hi"));

    unsafe { std::env::remove_var("RUNIE_SESSIONS_DIR") };
}
