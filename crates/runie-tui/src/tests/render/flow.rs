//! Layer 3 rendering tests for full agent tool flows.
//! Migrated from runie-core's `format_test` ASCII renderer → TUI TestBackend.

use super::*;
use super::super::*;
use runie_core::Event;
use runie_core::Role;

/// Helper: dispatch multiple events to simulate a tool flow
fn dispatch(state: &mut AppState, events: &[Event]) {
    for e in events {
        state.update(e.clone());
    }
}

/// Assert the rendered buffer contains key elements from a list_files tool flow.
fn assert_tool_flow_renders(state: &mut AppState) {
    let backend = TestBackend::new(80, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, state)).unwrap();
    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(
        content.contains("list_files"),
        "Should contain 'list_files': {}",
        content
    );
    assert!(content.contains("✓"), "Should contain '✓': {}", content);
    assert!(
        content.contains("→"),
        "Should contain agent arrow '→': {}",
        content
    );
}

// ─── test_formatted_labels_short_names ────────────────────────────────────────

/// Verifies that after a tool flow (thinking → tool → thinking → response →
/// turn_complete), the rendered view contains the expected labels.
#[test]
fn test_formatted_labels_short_names() {
    let backend = TestBackend::new(80, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut state = AppState::default();
    state.agent.streaming = true;
    dispatch(
        &mut state,
        &[
            Event::Thinking { id: "req.0".into() },
            Event::ThoughtDone { id: "req.0".into() },
            Event::ToolStart {
                id: "req.0".into(),
                name: "list_files".into(),
                input: serde_json::Value::Null,
            },
            Event::ToolEnd {
                id: "".to_string(),
                duration_secs: 0.3,
                output: String::new(),
            },
            Event::TurnComplete {
                id: "req.0".into(),
                duration_secs: 5.1,
            },
        ],
    );
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(content.contains("✓"), "Missing '✓' in: {}", content);
    assert!(
        content.contains("0.3s") || content.contains("0.3"),
        "Missing duration in: {}",
        content
    );
    assert!(
        content.contains("Turn completed") || content.contains("Turn"),
        "Missing 'Turn completed' in: {}",
        content
    );
}

// ─── test_list_files_full_tool_flow_sequence ──────────────────────────────────

fn full_tool_flow_events() -> Vec<Event> {
    vec![
        Event::Thinking { id: "req.0".into() },
        Event::ThoughtDone { id: "req.0".into() },
        Event::ToolStart {
            id: "req.0".into(),
            name: "list_files".into(),
            input: serde_json::Value::Null,
        },
        Event::ToolEnd {
            id: "".to_string(),
            duration_secs: 0.5,
            output: String::new(),
        },
        Event::Thinking { id: "req.0".into() },
        Event::ThoughtDone { id: "req.0".into() },
        Event::Response {
            id: "req.0".into(),
            content: "Here are the files:".into(),
        },
        Event::TurnComplete {
            id: "req.0".into(),
            duration_secs: 5.1,
        },
    ]
}

/// Verifies the full tool flow renders all expected elements.
#[test]
fn test_list_files_full_tool_flow_sequence() {
    let mut state = AppState::default();
    state.agent.streaming = true;
    dispatch(&mut state, &full_tool_flow_events());

    assert_eq!(state.session.messages.len(), 5);
    assert_eq!(state.session.messages[0].role, Role::Thought);
    assert_eq!(state.session.messages[1].role, Role::Tool);
    assert_eq!(state.session.messages[2].role, Role::Thought);
    assert_eq!(state.session.messages[3].role, Role::Assistant);
    assert_eq!(state.session.messages[4].role, Role::TurnComplete);

    assert_tool_flow_renders(&mut state);
}
