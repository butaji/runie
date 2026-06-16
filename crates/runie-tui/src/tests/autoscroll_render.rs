use super::*;
use runie_core::event::{AgentEvent, InputEvent};

fn render_chat(state: &mut AppState, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|f| view(f, state)).expect("draw");
    let buf = terminal.backend().buffer();
    buf.content.iter().map(|c| c.symbol()).collect()
}

#[test]
fn latest_user_message_visible_after_submit() {
    let mut state = AppState::default();
    state.input.input = "list files".into();
    state.update(Event::Input(InputEvent::Submit));
    state.ensure_fresh();

    let out = render_chat(&mut state, 40, 15);
    assert!(
        out.contains("list files"),
        "Submitted message must be visible"
    );
}

#[test]
fn large_tool_output_latest_visible_at_bottom() {
    let mut state = AppState::default();

    state.update(Event::Agent(AgentEvent::ToolStart { id: "req.0".into(), name: "ls".into(), input: serde_json::Value::Null }));
    let output = (1..=20)
        .map(|i| format!("file{}.txt", i))
        .collect::<Vec<_>>()
        .join("\n");
    state.update(Event::Agent(AgentEvent::ToolEnd {
        id: "".to_string(),
        duration_secs: 0.5,
        output,
    }));
    state.ensure_fresh();
    state.view.scroll = 0;

    let out = render_chat(&mut state, 40, 20);
    assert!(
        out.contains("file20"),
        "Latest file (file20) must be visible at bottom"
    );
}

#[test]
fn final_response_visible_after_full_turn() {
    let mut state = AppState::default();
    state.agent.streaming = true;

    state.update(Event::Agent(AgentEvent::Thinking { id: "req.0".into() }));
    state.update(Event::Agent(AgentEvent::Response {
        id: "req.0".into(),
        content: "I'll list files.\nTOOL:list_dir:.".into(),
    }));
    state.update(Event::Agent(AgentEvent::ThoughtDone { id: "req.0".into() }));
    state.update(Event::Agent(AgentEvent::ToolStart { id: "req.0".into(), name: "list_dir".into(), input: serde_json::Value::Null }));
    let output = (1..=15)
        .map(|i| format!("file{}.txt", i))
        .collect::<Vec<_>>()
        .join("\n");
    state.update(Event::Agent(AgentEvent::ToolEnd {
        id: "".to_string(),
        duration_secs: 0.5,
        output,
    }));
    state.update(Event::Agent(AgentEvent::Response {
        id: "req.0".into(),
        content: "Done!".into(),
    }));
    state.update(Event::Agent(AgentEvent::TurnComplete {
        id: "req.0".into(),
        duration_secs: 2.0,
    }));
    state.update(Event::Agent(AgentEvent::Done { id: "req.0".into() }));
    state.ensure_fresh();
    state.view.scroll = 0;

    let out = render_chat(&mut state, 40, 20);
    assert!(out.contains("Done!"), "Final 'Done!' must be visible");
}

#[test]
fn latest_message_pushes_older_off_screen() {
    let mut state = AppState::default();

    for i in 0..15 {
        state.input.input = format!("msg{}", i);
        state.update(Event::Input(InputEvent::Submit));
    }
    state.ensure_fresh();
    state.view.scroll = 0;

    let out = render_chat(&mut state, 40, 15);
    if !out.contains("msg14") {
        eprintln!(
            "BUFFER:\n{}",
            out.chars()
                .collect::<Vec<_>>()
                .chunks(40)
                .map(|c| c.iter().collect::<String>())
                .collect::<Vec<_>>()
                .join("\n")
        );
        eprintln!(
            "total_lines={} scroll={}",
            state.view.total_lines, state.view.scroll
        );
    }
    assert!(!out.contains("msg0"), "Oldest message should be off-screen");
    assert!(out.contains("msg14"), "Latest message must be visible");
}

#[test]
fn scroll_up_shows_older_content() {
    let mut state = AppState::default();
    for i in 0..15 {
        state.input.input = format!("msg{}", i);
        state.update(Event::Input(InputEvent::Submit));
    }
    state.ensure_fresh();
    state.view.scroll = 100; // clamped to max

    let out = render_chat(&mut state, 40, 15);
    assert!(
        out.contains("msg0"),
        "Oldest message visible after scroll up"
    );
    assert!(
        !out.contains("msg14"),
        "Latest message hidden after scroll up"
    );
}

#[test]
fn new_content_auto_shows_when_at_bottom() {
    let mut state = AppState::default();
    for i in 0..10 {
        state.input.input = format!("msg{}", i);
        state.update(Event::Input(InputEvent::Submit));
    }
    state.ensure_fresh();
    state.view.scroll = 0;

    state.input.input = "NEWEST".into();
    state.update(Event::Input(InputEvent::Submit));
    state.ensure_fresh();

    let out = render_chat(&mut state, 40, 15);
    assert!(
        out.contains("NEWEST"),
        "New content must be visible when at bottom"
    );
}
