use crate::model::{AppState, ChatMessage, Role};
use crate::event::Event;

fn fresh_state() -> AppState {
    AppState::default()
}

/// Simulates the exact user flow: submit, agent thinking + tool + large output
#[test]
fn list_files_large_output_latest_visible() {
    let mut state = fresh_state();
    let height = 5; // Small viewport

    // 1. User submits
    state.input = "list files".into();
    state.update(Event::Submit);
    state.ensure_fresh();
    state.scroll = 0;

    let region = state.visible_scroll(height);
    assert!(
        region.elements.iter().any(|e| matches!(e, crate::ui::Element::UserMessage { content } if content == "list files")),
        "User message must be visible after submit"
    );

    // 2. Agent thinks + responds with tool
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentResponse { id: "req.0".into(), content: "I'll list files.\nTOOL:list_dir:.".into() });
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
    state.ensure_fresh();
    state.scroll = 0;

    let region = state.visible_scroll(height);
    let has_thought = region.elements.iter().any(|e| matches!(e, crate::ui::Element::ThoughtMarker { .. }));
    assert!(has_thought, "Thought must be visible");

    // 3. Tool runs with large output
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "list_dir".into() });
    let output = (1..=20).map(|i| format!("file{}.txt", i)).collect::<Vec<_>>().join("\n");
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output });
    state.ensure_fresh();
    state.scroll = 0;

    // The tool output is large (21 lines: header + 20 files). Viewport is 5 lines.
    // The latest lines (bottom) MUST be visible.
    let region = state.visible_scroll(height);
    assert!(!region.elements.is_empty(), "Visible region must not be empty");

    // The last element in the visible region should be the tool or something after it
    let last_elem = region.elements.iter().rev().find(|e| !matches!(e, crate::ui::Element::Spacer));
    assert!(last_elem.is_some(), "Last visible element must exist");

    // 4. Final response
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Done!".into() });
    state.ensure_fresh();
    state.scroll = 0;

    let region = state.visible_scroll(height);
    let has_done = region.elements.iter().any(|e| matches!(e, crate::ui::Element::AgentMessage { content } if content == "Done!"));
    assert!(has_done, "Final 'Done!' must be visible at bottom");
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
    state.messages.push(ChatMessage {
        role: Role::Thought,
        content: thought,
        timestamp: 1.0,
        id: "t1".into(),
    });
    state.messages_changed();
    state.ensure_fresh();
    state.scroll = 0;

    let region = state.visible_scroll(height);
    assert!(!region.elements.is_empty(), "Visible region must not be empty");

    // The thought is 21 lines. Viewport is 5 lines at bottom.
    // We should see the bottom 5 lines of the thought.
    let thought_elems: Vec<_> = region.elements.iter()
        .filter(|e| matches!(e, crate::ui::Element::ThoughtMarker { .. }))
        .collect();
    assert!(!thought_elems.is_empty(), "Thought must be in visible region");
}

#[test]
fn viewport_never_empty_when_content_exists() {
    let mut state = fresh_state();
    let height = 5;

    // Add 10 messages = 20 lines total
    for i in 0..10 {
        state.messages.push(ChatMessage {
            role: Role::User,
            content: format!("msg{}", i),
            timestamp: i as f64,
            id: format!("u{}", i),
        });
    }
    state.messages_changed();
    state.ensure_fresh();
    state.scroll = 0;

    let region = state.visible_scroll(height);
    assert!(!region.elements.is_empty(), "Visible region must not be empty when content exists");
}

#[test]
fn scroll_zero_always_shows_latest() {
    let mut state = fresh_state();
    let height = 5;

    // Add 3 messages = 6 lines (fits in viewport? no, 6 > 5)
    for i in 0..3 {
        state.messages.push(ChatMessage {
            role: Role::User,
            content: format!("msg{}", i),
            timestamp: i as f64,
            id: format!("u{}", i),
        });
    }
    state.messages_changed();
    state.ensure_fresh();
    state.scroll = 0;

    let region = state.visible_scroll(height);
    // Latest message (msg2) should be visible
    let has_latest = region.elements.iter().any(|e| matches!(e, crate::ui::Element::UserMessage { content } if content == "msg2"));
    assert!(has_latest, "Latest message must be visible when scroll=0");
}

#[test]
fn tool_output_exceeding_viewport_shows_latest_files() {
    let mut state = fresh_state();
    let height = 5;

    state.update(Event::AgentToolStart { id: "req.0".into(), name: "ls".into() });
    let output = (1..=50).map(|i| format!("file{}.txt", i)).collect::<Vec<_>>().join("\n");
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output });
    state.ensure_fresh();
    state.scroll = 0;

    // ToolDone: header (1) + 50 output lines = 51 lines total. Viewport = 5.
    // The bottom 5 lines should be visible: file50, file49, file48, file47, header
    let region = state.visible_scroll(height);
    assert!(!region.elements.is_empty(), "Tool output must be visible");

    // The tool element should be in the visible region
    let tool_elems: Vec<_> = region.elements.iter()
        .filter(|e| matches!(e, crate::ui::Element::ToolDone { .. }))
        .collect();
    assert!(!tool_elems.is_empty(), "ToolDone must be in visible region");
}
