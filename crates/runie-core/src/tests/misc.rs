use crate::model::AppState;
use crate::event::Event;
use crate::ui::format_messages;

fn fresh_state() -> AppState {
    AppState::default()
}

#[test]
fn test_reset_clears_state() {
    let mut state = fresh_state();
    state.input = "test".to_string();
    state.streaming = true;

    state.update(Event::Reset);

    assert_eq!(state.input, "");
    assert!(!state.streaming);
    assert_eq!(state.messages.len(), 0);
}

#[test]
fn test_scroll_up() {
    let mut state = fresh_state();
    state.update(Event::ScrollUp);
    assert_eq!(state.scroll, 1);
}

#[test]
fn test_scroll_down() {
    let mut state = fresh_state();
    state.scroll = 5;
    state.update(Event::ScrollDown);
    assert_eq!(state.scroll, 4);
}

#[test]
fn test_scroll_down_saturates() {
    let mut state = fresh_state();
    state.scroll = 0;
    state.update(Event::ScrollDown);
    assert_eq!(state.scroll, 0);
}

#[test]
fn test_tool_flow_creates_two_thoughts() {
    let mut state = fresh_state();
    state.streaming = true;

    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
    state.update(Event::AgentToolStart {
        id: "req.0".to_string(),
        name: "list_files".to_string(),
    });
    state.update(Event::AgentToolEnd { duration_secs: 0.5 });
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
    state.update(Event::AgentResponse {
        id: "req.0".to_string(),
        content: "Here are the files".to_string()
    });

    let thought_count = state.messages.iter().filter(|m| m.role == "thought").count();
    assert_eq!(thought_count, 2);
}

#[test]
fn test_turn_complete_event() {
    let mut state = fresh_state();
    state.has_intermediate_steps = true;

    state.update(Event::AgentTurnComplete {
        id: "req.0".to_string(),
        duration_secs: 5.1
    });

    assert_eq!(state.messages.len(), 1);
    let msg = &state.messages[0];
    assert_eq!(msg.role, "turn_complete");
    assert!(msg.content.contains("5.1s"));
}

#[test]
fn test_turn_complete_not_shown_for_simple_flow() {
    let mut state = fresh_state();
    state.streaming = true;

    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
    state.update(Event::AgentResponse {
        id: "req.0".to_string(),
        content: "Hi".to_string()
    });
    state.update(Event::AgentTurnComplete {
        id: "req.0".to_string(),
        duration_secs: 1.0
    });

    let has_turn_complete = state.messages.iter().any(|m| m.role == "turn_complete");
    assert!(!has_turn_complete, "Simple flow should not show turn complete");
}

#[test]
fn test_tool_done_event() {
    let mut state = fresh_state();
    state.update(Event::AgentToolStart {
        id: "req.0".to_string(),
        name: "list_files".to_string(),
    });
    state.update(Event::AgentToolEnd { duration_secs: 0.3 });

    assert_eq!(state.messages.len(), 1);
    let msg = &state.messages[0];
    assert_eq!(msg.role, "tool");
    assert!(msg.content.contains("list_files"));
    assert!(msg.content.contains("0.3s"));
}

#[test]
fn test_formatted_labels_short_names() {
    let mut state = fresh_state();
    state.streaming = true;

    state.update(Event::AgentToolStart {
        id: "req.0".to_string(),
        name: "list_files".to_string(),
    });
    state.update(Event::AgentToolEnd { duration_secs: 0.3 });
    state.update(Event::AgentTurnComplete {
        id: "req.0".to_string(),
        duration_secs: 5.1
    });

    let lines = format_messages(&state);
    let content: String = lines.iter()
        .flat_map(|l| l.spans.iter().map(|s| s.text.clone()).collect::<Vec<_>>())
        .collect();

    assert!(content.contains("Ran"), "Missing 'Ran' in: {}", content);
    assert!(content.contains("0.3s"), "Missing '0.3s' in: {}", content);
    assert!(content.contains("Turn completed"), "Missing 'Turn completed' in: {}", content);
}

#[test]
fn test_list_files_full_tool_flow_sequence() {
    let mut state = fresh_state();
    state.streaming = true;

    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
    state.update(Event::AgentToolStart {
        id: "req.0".to_string(),
        name: "list_files".to_string(),
    });
    state.update(Event::AgentToolEnd { duration_secs: 0.5 });
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
    state.update(Event::AgentResponse {
        id: "req.0".to_string(),
        content: "Here are the files:".to_string()
    });
    state.update(Event::AgentTurnComplete {
        id: "req.0".to_string(),
        duration_secs: 5.1
    });

    assert_eq!(state.messages.len(), 5);
    assert_eq!(state.messages[0].role, "thought");
    assert_eq!(state.messages[1].role, "tool");
    assert_eq!(state.messages[2].role, "thought");
    assert_eq!(state.messages[3].role, "assistant");
    assert_eq!(state.messages[4].role, "turn_complete");

    let lines = format_messages(&state);
    let content: String = lines.iter()
        .flat_map(|l| l.spans.iter().map(|s| s.text.clone()).collect::<Vec<_>>())
        .collect();

    assert!(content.contains("Though"));
    assert!(content.contains("Ran"));
    assert!(content.contains("list_files"));
    assert!(content.contains("Agent:"));
    assert!(content.contains("Turn completed in 5.1s"));
}

#[test]
fn test_thinking_indicator_shows_for_queued_request() {
    let mut state = fresh_state();
    state.streaming = true;
    state.request_queue.push(("B".to_string(), "req.1".to_string()));
    state.thinking_started_at = Some(std::time::Instant::now());

    let has_thought = state.messages.iter().any(|m| m.role == "thought");
    assert!(!has_thought);
}
