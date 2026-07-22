#![allow(clippy::too_many_lines)]
use super::*;
use runie_core::Event;

/// Render at 60x20 for this test suite.
fn render_content(state: &mut AppState) -> String {
    render_with_size(state, 60, 20)
}

fn add_messages(state: &mut AppState, count: usize) {
    for i in 0..count {
        state.update(Event::Response {
            id: format!("req.{}", i),
            content: format!("msg{}", i),
            role: String::new(),
            timestamp: 0.0,
            provider: String::new(),
        });
    }
    state.ensure_fresh();
}

#[test]
fn latest_message_visible_after_submit() {
    let mut state = AppState::default();
    add_messages(&mut state, 30);
    state.view.scroll = 5; // scrolled up

    state.input.input = "hello".to_string();
    state.update(Event::Submit);
    state.ensure_fresh();

    let out = render_content(&mut state);
    assert!(
        out.contains("hello"),
        "Submitted message must be visible at bottom"
    );
}

#[test]
fn latest_agent_response_visible_when_at_bottom() {
    let mut state = AppState::default();
    add_messages(&mut state, 30);
    state.view.scroll = 0; // at bottom

    state.update(Event::Response {
        id: "req.99".to_string(),
        content: "Latest response".to_string(),
        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),
    });
    state.ensure_fresh();

    let out = render_content(&mut state);
    assert!(
        out.contains("Latest response"),
        "Agent response must be visible when at bottom"
    );
}

#[test]
fn latest_thought_visible_when_at_bottom() {
    let mut state = AppState::default();
    add_messages(&mut state, 30);
    state.view.scroll = 0;

    state.update(Event::Thinking { id: "req.0".to_string() });
    state.update(Event::Response {
        id: "req.0".to_string(),
        content: "I'll list files.\n".to_string(),
        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),
    });
    state.update(Event::Response {
        id: "req.0".to_string(),
        content: "TOOL:list_dir:.".to_string(),
        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),
    });
    state.update(Event::ThoughtDone { id: "req.0".to_string() });
    // Thoughts are summarized by default; expand the thought post (the
    // latest, bottom post) so its reasoning participates in sticky-bottom.
    let thought_idx = state.snapshot().posts.len() - 1;
    state.view.expanded_posts.insert(thought_idx);
    state.messages_changed();
    state.ensure_fresh();

    let out = render_content(&mut state);
    assert!(
        out.contains("I'll list files"),
        "Thought reasoning must be visible when at bottom"
    );
}

#[test]
fn latest_tool_visible_when_at_bottom() {
    let mut state = AppState::default();
    add_messages(&mut state, 30);
    state.view.scroll = 0;

    state.update(Event::ToolStart {
        id: "req.0".to_string(),
        name: "list_dir".to_string(),
        input: serde_json::Value::Null,
    });
    state.update(Event::ToolEnd {
        id: "".to_string(),
        input: None,
        duration_secs: 0.5,
        output: "file1\nfile2\nfile3".to_string(),
    });
    state.ensure_fresh();

    let out = render_content(&mut state);
    assert!(
        out.contains("file3"),
        "Latest tool output line must be visible when at bottom"
    );
}

#[test]
fn sticky_bottom_clips_top_not_bottom() {
    let mut state = AppState::default();
    add_messages(&mut state, 5);
    state.update(Event::Thinking { id: "req.99".to_string() });
    state.update(Event::Response { id: "req.99".to_string(), content: "Reasoning line1\nline2\nline3\nline4\nline5\nline6\nline7\nline8\nline9\nline10\nline11\nline12\nline13\nline14\nline15\n".to_string(), role: String::new(), timestamp: 0.0, provider: String::new() });
    state.update(Event::Response {
        id: "req.99".to_string(),
        content: "TOOL:list_dir:.".to_string(),
        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),
    });
    state.update(Event::ThoughtDone { id: "req.99".to_string() });
    // Thoughts are summarized by default; expand the thought post (the
    // latest, bottom post) so its body overflows the viewport.
    let thought_idx = state.snapshot().posts.len() - 1;
    state.view.expanded_posts.insert(thought_idx);
    state.messages_changed();
    state.ensure_fresh();
    state.view.scroll = 0;

    let out = render_content(&mut state);
    // With sticky bottom, the BOTTOM lines of overflow content should be visible.
    assert!(
        out.contains("line15"),
        "Bottom lines of overflow content must be visible"
    );
}

#[test]
fn user_scrolled_up_does_not_see_new_content() {
    let mut state = AppState::default();
    add_messages(&mut state, 30);
    state.view.scroll = 10; // scrolled up

    state.update(Event::Response {
        id: "req.99".to_string(),
        content: "Hidden response".to_string(),
        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),
    });
    state.ensure_fresh();

    // When scrolled up, new content may not be visible. Key: scroll position preserved.
    assert_eq!(
        state.view.scroll, 10,
        "Scroll position should be preserved when user is not at bottom"
    );
}

#[test]
fn scroll_down_to_bottom_shows_latest() {
    let mut state = AppState::default();
    add_messages(&mut state, 30);
    state.view.scroll = 10;

    // Scroll down back to bottom
    for _ in 0..15 {
        state.update(Event::Down);
    }
    assert_eq!(state.view.scroll, 0, "ScrollDown should reach bottom");

    let out = render_content(&mut state);
    assert!(
        out.contains("msg29"),
        "Latest message should be visible after scrolling to bottom"
    );
}

#[test]
fn mixed_content_latest_visible() {
    let mut state = AppState::default();
    add_messages(&mut state, 20);
    state.update(Event::Thinking { id: "req.0".to_string() });
    state.update(Event::Response {
        id: "req.0".to_string(),
        content: "◆ Thought 1.0s\nReasoning line 1\nReasoning line 2".to_string(),
        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),
    });
    state.update(Event::ThoughtDone { id: "req.0".to_string() });
    state.update(Event::ToolStart { id: "req.0".to_string(), name: "ls".to_string(), input: serde_json::Value::Null });
    state.update(Event::ToolEnd {
        id: "".to_string(),
        input: None,
        duration_secs: 0.5,
        output: "file1\nfile2".to_string(),
    });
    state.update(Event::Response {
        id: "req.0".to_string(),
        content: "Done!".to_string(),
        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),
    });
    state.ensure_fresh();
    state.view.scroll = 0;

    let out = render_content(&mut state);
    assert!(
        out.contains("Done!"),
        "Latest assistant message must be visible"
    );
    assert!(out.contains("file2"), "Latest tool output must be visible");
}

#[test]
fn empty_chat_shows_panels() {
    let mut state = AppState::default();
    state.ensure_fresh();

    let out = render_content(&mut state);
    if runie_core::provider::is_mock_enabled() {
        assert!(
            out.contains("mock/echo"),
            "Input panel should show mock/echo in dev"
        );
    }
}
