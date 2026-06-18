//! Tool output truncation tests.

use crate::event::AgentEvent;
use crate::model::AppState;

#[test]
fn tool_output_truncated_to_max_lines() {
    let mut state = AppState::default();
    state.config.truncation.max_lines = 3;
    state.config.truncation.max_bytes = 100_000;

    state.update(AgentEvent::ToolStart {
        id: "req.0".into(),
        name: "bash".into(),
        input: serde_json::Value::Null,
    });
    state.update(AgentEvent::ToolEnd {
        id: "".to_string(),
        duration_secs: 0.5,
        output: (0..10)
            .map(|i| format!("line {}", i))
            .collect::<Vec<_>>()
            .join("\n"),
    });

    let tool_msg = state
        .session
        .messages
        .iter()
        .find(|m| m.role == crate::model::Role::Tool)
        .expect("tool message should exist");
    assert!(
        tool_msg.content.contains('…'),
        "Tool output should be truncated: {}",
        tool_msg.content
    );
    assert!(
        !tool_msg.content.contains("line 9"),
        "Truncated output should not contain late lines"
    );
}

#[test]
fn tool_output_truncated_to_max_bytes() {
    let mut state = AppState::default();
    state.config.truncation.max_lines = 10_000;
    state.config.truncation.max_bytes = 10;

    state.update(AgentEvent::ToolStart {
        id: "req.0".into(),
        name: "bash".into(),
        input: serde_json::Value::Null,
    });
    state.update(AgentEvent::ToolEnd {
        id: "".to_string(),
        duration_secs: 0.5,
        output: "this is a long output".into(),
    });

    let tool_msg = state
        .session
        .messages
        .iter()
        .find(|m| m.role == crate::model::Role::Tool)
        .expect("tool message should exist");
    assert!(
        tool_msg.content.contains('…'),
        "Tool output should be truncated by bytes: {}",
        tool_msg.content
    );
}
