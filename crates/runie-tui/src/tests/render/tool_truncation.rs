//! Layer 3 rendering tests for tool output truncation.

use ratatui::{backend::TestBackend, Terminal};

use runie_core::Event;

fn render_chat(state: &mut runie_core::AppState, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|f| crate::ui::view(f, state)).expect("draw");
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|c| c.symbol())
        .collect()
}

#[test]
fn long_tool_output_truncated_with_ellipsis() {
    let mut state = runie_core::AppState::default();
    state.config.truncation.max_lines = 3;
    state.config.truncation.max_bytes = 10_000;

    state.update(Event::ToolStart {
        id: "req.0".into(),
        name: "bash".into(),
        input: serde_json::Value::Null,
    });
    let output = (0..20)
        .map(|i| format!("line {}", i))
        .collect::<Vec<_>>()
        .join("\n");
    state.update(Event::ToolEnd {
        id: "".to_string(),
        input: None,
        duration_secs: 0.5,
        output,
    });
    state.ensure_fresh();
    state.view.scroll = 0;

    let out = render_chat(&mut state, 40, 20);
    assert!(
        out.contains('…'),
        "Truncated tool output should contain ellipsis: {}",
        out
    );
}

#[test]
fn truncated_tool_output_keeps_header() {
    let mut state = runie_core::AppState::default();
    state.config.truncation.max_lines = 2;
    state.config.truncation.max_bytes = 10_000;

    state.update(Event::ToolStart {
        id: "req.0".into(),
        name: "list_files".into(),
        input: serde_json::Value::Null,
    });
    state.update(Event::ToolEnd {
        id: "".to_string(),
        input: None,
        duration_secs: 0.5,
        output: (0..10)
            .map(|i| format!("file{}", i))
            .collect::<Vec<_>>()
            .join("\n"),
    });
    state.ensure_fresh();
    state.view.scroll = 0;

    let out = render_chat(&mut state, 40, 20);
    assert!(
        out.contains("list_files"),
        "Tool header should still render after truncation: {}",
        out
    );
}
