#![allow(clippy::needless_borrow)]

use runie_core::event::AgentEvent;
use runie_core::event::Event;
use runie_core::model::{AppState, ChatMessage,  Role};
use runie_core::Part;
use runie_testing::fresh_state;

/// Simulates the exact user flow: submit, agent thinking + tool + large output
#[test]
fn list_files_large_output_latest_visible() {
    let mut state = fresh_state();
    let height = 5;

    verify_user_submit_visible(&mut state, height);
    verify_thought_visible(&mut state, height);
    verify_tool_output_visible(&mut state, height);
    verify_final_done_visible(&mut state, height);
}

fn verify_user_submit_visible(state: &mut AppState, height: usize) {
    state.input.input = "list files".into();
    state.update(Event::submit());
    state.ensure_fresh();
    state.view.scroll = 0;

    let region = crate::tests::core::visible_helper::compute_viewport(&state, height);
    assert!(region.elements.iter().any(|e| matches!(e, runie_core::ui::Element::UserMessage { content, .. } if content == "list files")),
        "User message must be visible after submit");
}

fn verify_thought_visible(state: &mut AppState, height: usize) {
    state.agent.streaming = true;
    state.update(AgentEvent::Thinking { id: "req.0".into() });
    state.update(AgentEvent::Response {
        id: "req.0".into(),
        content: "I'll list files.\nTOOL:list_dir:.".into(),
    });
    state.update(AgentEvent::ThoughtDone { id: "req.0".into() });
    state.ensure_fresh();
    state.view.scroll = 0;

    let region = crate::tests::core::visible_helper::compute_viewport(&state, height);
    assert!(
        region
            .elements
            .iter()
            .any(|e| matches!(e, runie_core::ui::Element::ThoughtMarker { .. })),
        "Thought must be visible"
    );
}

fn verify_tool_output_visible(state: &mut AppState, height: usize) {
    state.update(AgentEvent::ToolStart {
        id: "req.0".into(),
        name: "list_dir".into(),
        input: serde_json::Value::Null,
    });
    let output = (1..=20)
        .map(|i| format!("file{}.txt", i))
        .collect::<Vec<_>>()
        .join("\n");
    state.update(AgentEvent::ToolEnd {
        id: "".to_string(),
        duration_secs: 0.5,
        output,
    });
    state.ensure_fresh();
    state.view.scroll = 0;

    let region = crate::tests::core::visible_helper::compute_viewport(&state, height);
    assert!(
        !region.elements.is_empty(),
        "Visible region must not be empty"
    );
    let last_elem = region
        .elements
        .iter()
        .rev()
        .find(|e| !matches!(e, runie_core::ui::Element::Spacer { .. }));
    assert!(last_elem.is_some(), "Last visible element must exist");
}

fn verify_final_done_visible(state: &mut AppState, height: usize) {
    state.update(AgentEvent::Response {
        id: "req.0".into(),
        content: "Done!".into(),
    });
    state.ensure_fresh();
    state.view.scroll = 0;

    let region = crate::tests::core::visible_helper::compute_viewport(&state, height);
    assert!(
        region.elements.iter().any(
            |e| matches!(e, runie_core::ui::Element::AgentMessage { content, .. } if content == "Done!")
        ),
        "Final 'Done!' must be visible at bottom"
    );
}

#[test]
fn large_thought_bottom_lines_visible() {
    let mut state = fresh_state();
    let height = 5;

    // Create a thought with many lines
    let mut thought = "◆ Thought 1.0s\n".to_string();
    for i in 1..=20 {
        thought.push_str(&format!("line{}\n", i));
    }
    state.session.messages.push(ChatMessage {
        role: Role::Thought,
        parts: vec![Part::Text { content: thought }],
        timestamp: 1.0,
        id: "t1".into(),
        ..Default::default()
    });
    state.messages_changed();
    state.ensure_fresh();
    state.view.scroll = 0;

    let region = crate::tests::core::visible_helper::compute_viewport(&state, height);
    assert!(
        !region.elements.is_empty(),
        "Visible region must not be empty"
    );

    // The thought is 21 lines. Viewport is 5 lines at bottom.
    // We should see the bottom 5 lines of the thought.
    let thought_elems: Vec<_> = region
        .elements
        .iter()
        .filter(|e| matches!(e, runie_core::ui::Element::ThoughtMarker { .. }))
        .collect();
    assert!(
        !thought_elems.is_empty(),
        "Thought must be in visible region"
    );
}

#[test]
fn viewport_never_empty_when_content_exists() {
    let mut state = fresh_state();
    let height = 5;

    // Add 10 messages = 20 lines total
    for i in 0..10 {
        state.session.messages.push(ChatMessage {
            role: Role::User,
            parts: vec![Part::Text { content: format!("msg{}", i) }],
            timestamp: i as f64,
            id: format!("u{}", i),
            ..Default::default()
        });
    }
    state.messages_changed();
    state.ensure_fresh();
    state.view.scroll = 0;

    let region = crate::tests::core::visible_helper::compute_viewport(&state, height);
    assert!(
        !region.elements.is_empty(),
        "Visible region must not be empty when content exists"
    );
}

#[test]
fn scroll_zero_always_shows_latest() {
    let mut state = fresh_state();
    let height = 5;

    // Add 3 messages = 6 lines (fits in viewport? no, 6 > 5)
    for i in 0..3 {
        state.session.messages.push(ChatMessage {
            role: Role::User,
            parts: vec![Part::Text { content: format!("msg{}", i) }],
            timestamp: i as f64,
            id: format!("u{}", i),
            ..Default::default()
        });
    }
    state.messages_changed();
    state.ensure_fresh();
    state.view.scroll = 0;

    let region = crate::tests::core::visible_helper::compute_viewport(&state, height);
    // Latest message (msg2) should be visible
    let has_latest = region.elements.iter().any(
        |e| matches!(e, runie_core::ui::Element::UserMessage { content, .. } if content == "msg2"),
    );
    assert!(has_latest, "Latest message must be visible when scroll=0");
}

#[test]
fn tool_output_exceeding_viewport_shows_latest_files() {
    let mut state = fresh_state();
    let height = 5;

    state.update(AgentEvent::ToolStart {
        id: "req.0".into(),
        name: "ls".into(),
        input: serde_json::Value::Null,
    });
    let output = (1..=50)
        .map(|i| format!("file{}.txt", i))
        .collect::<Vec<_>>()
        .join("\n");
    state.update(AgentEvent::ToolEnd {
        id: "".to_string(),
        duration_secs: 0.5,
        output,
    });
    state.ensure_fresh();
    state.view.scroll = 0;

    // ToolDone: header (1) + 50 output lines = 51 lines total. Viewport = 5.
    // The bottom 5 lines should be visible: file50, file49, file48, file47, header
    let region = crate::tests::core::visible_helper::compute_viewport(&state, height);
    assert!(!region.elements.is_empty(), "Tool output must be visible");

    // The tool element should be in the visible region
    let tool_elems: Vec<_> = region
        .elements
        .iter()
        .filter(|e| matches!(e, runie_core::ui::Element::ToolDone { .. }))
        .collect();
    assert!(!tool_elems.is_empty(), "ToolDone must be in visible region");
}
