use super::*;
use runie_core::event::AgentEvent;

fn render_content(state: &mut AppState) -> String {
    render_with_height(state, 10)
}

fn render_with_height(state: &mut AppState, height: u16) -> String {
    let backend = TestBackend::new(60, height);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|f| view(f, state)).expect("draw");
    let buf = terminal.backend().buffer();
    buf.content.iter().map(|c| c.symbol()).collect()
}

#[test]
fn latest_message_visible_at_bottom() {
    let mut state = AppState::default();
    for i in 0..8 {
        state.update(AgentEvent::Response {
            id: format!("req.{}", i),
            content: format!("msg{}", i),
        });
    }
    state.ensure_fresh();
    state.view.scroll = 0;

    let out = render_content(&mut state);
    assert!(
        out.contains("msg7"),
        "Latest message (msg7) must be visible at bottom"
    );
}

#[test]
fn large_thought_clipped_from_top_not_bottom() {
    let mut state = AppState::default();
    state.update(AgentEvent::Response {
        id: "req.0".into(),
        content: "before".into(),
    });
    state.update(AgentEvent::Thinking { id: "req.0".into() });
    let mut thought = "◆ Thought 1.0s\n".to_string();
    for i in 1..=15 {
        thought.push_str(&format!("line{}\n", i));
    }
    state.update(AgentEvent::Response {
        id: "req.0".into(),
        content: thought,
    });
    state.update(AgentEvent::ThoughtDone { id: "req.0".into() });
    state.update(AgentEvent::Response {
        id: "req.0".into(),
        content: "after".into(),
    });
    state.ensure_fresh();
    state.view.scroll = 0;

    let out = render_with_height(&mut state, 30);
    assert!(
        out.contains("after"),
        "Latest content (after) must be visible"
    );
    assert!(
        out.contains("line15"),
        "Bottom lines of thought should be visible"
    );
}

#[test]
fn scroll_up_shows_older_content() {
    let mut state = AppState::default();
    for i in 0..20 {
        state.update(AgentEvent::Response {
            id: format!("req.{}", i),
            content: format!("msg{}", i),
        });
    }
    state.ensure_fresh();
    // 20 messages = 40 lines. With 15-row terminal, chat panel has ~9 inner lines.
    // Scroll up enough to see oldest content.
    state.view.scroll = 100; // auto-clamped to max_scroll

    let out = render_with_height(&mut state, 15);
    assert!(
        out.contains("msg0"),
        "Oldest message should be visible after scrolling up"
    );
    assert!(
        !out.contains("msg19"),
        "Latest message should be hidden after scrolling up"
    );
}

#[test]
fn scrollbar_visible_when_content_overflows() {
    let mut state = AppState::default();
    for i in 0..20 {
        state.update(AgentEvent::Response {
            id: format!("req.{}", i),
            content: format!("msg{}", i),
        });
    }
    state.ensure_fresh();
    state.view.scroll = 0;

    let out = render_content(&mut state);
    assert!(
        out.contains("▐"),
        "Scrollbar thumb should be visible when content overflows"
    );
}

#[test]
fn tool_output_latest_lines_visible() {
    let mut state = AppState::default();
    state.update(AgentEvent::ToolStart {
        id: "req.0".into(),
        name: "ls".into(),
        input: serde_json::Value::Null,
    });
    let output = (1..=10)
        .map(|i| format!("file{}", i))
        .collect::<Vec<_>>()
        .join("\n");
    state.update(AgentEvent::ToolEnd {
        id: "".to_string(),
        duration_secs: 0.5,
        output,
    });
    state.ensure_fresh();
    state.view.scroll = 0;

    let out = render_with_height(&mut state, 30);
    assert!(
        out.contains("file10"),
        "Latest tool output (file10) must be visible"
    );
    assert!(out.contains("✓ Run ls"), "Tool header must be visible");
}

#[test]
fn new_message_pushes_old_upward() {
    let mut state = AppState::default();
    for i in 0..5 {
        state.update(AgentEvent::Response {
            id: format!("req.{}", i),
            content: format!("msg{}", i),
        });
    }
    state.ensure_fresh();
    let before = render_with_height(&mut state, 30);
    assert!(before.contains("msg0"), "msg0 visible before overflow");

    // Add more messages to overflow
    for i in 5..25 {
        state.update(AgentEvent::Response {
            id: format!("req.{}", i),
            content: format!("msg{}", i),
        });
    }
    state.ensure_fresh();
    let after = render_content(&mut state); // 10-row terminal = small viewport
    assert!(
        !after.contains("msg0"),
        "msg0 should be pushed off-screen by newer messages"
    );
    assert!(after.contains("msg24"), "Latest msg24 must be visible");
}

#[test]
fn partial_element_at_top_when_overflow() {
    let mut state = AppState::default();
    state.update(AgentEvent::Response {
        id: "req.0".into(),
        content: "first".into(),
    });
    let mut thought = "◆ Thought 1.0s\n".to_string();
    for i in 1..=20 {
        thought.push_str(&format!("line{}\n", i));
    }
    state.update(AgentEvent::Thinking { id: "req.1".into() });
    state.update(AgentEvent::Response {
        id: "req.1".into(),
        content: thought,
    });
    state.update(AgentEvent::ThoughtDone { id: "req.1".into() });
    state.update(AgentEvent::Response {
        id: "req.2".into(),
        content: "last".into(),
    });
    state.ensure_fresh();
    state.view.scroll = 0;

    let out = render_with_height(&mut state, 30);
    assert!(out.contains("last"), "Latest message must be visible");
    assert!(out.contains("line20"), "Bottom of thought visible");
}

#[test]
fn scroll_position_preserved_during_streaming() {
    let mut state = AppState::default();
    for i in 0..15 {
        state.update(AgentEvent::Response {
            id: format!("req.{}", i),
            content: format!("msg{}", i),
        });
    }
    state.ensure_fresh();
    state.view.scroll = 8; // user scrolled up reading history

    // New streaming content arrives
    state.update(AgentEvent::Response {
        id: "req.99".into(),
        content: "new".into(),
    });
    state.ensure_fresh();

    // Scroll should be preserved
    assert_eq!(
        state.view.scroll, 8,
        "Scroll position should be preserved when not at bottom"
    );
}

#[test]
fn at_bottom_auto_scrolls_to_show_new() {
    let mut state = AppState::default();
    for i in 0..15 {
        state.update(AgentEvent::Response {
            id: format!("req.{}", i),
            content: format!("msg{}", i),
        });
    }
    state.ensure_fresh();
    state.view.scroll = 0; // at bottom

    // Submit a new message
    state.input.input = "hello".to_string();
    state.update(runie_core::event::InputEvent::Submit);
    state.ensure_fresh();

    let out = render_content(&mut state);
    assert!(
        out.contains("hello"),
        "Newly submitted message must be visible"
    );
}
