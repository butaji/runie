//! Rendering tests for global expand/collapse (Ctrl+O).
//!
//! Ctrl+O toggles the global collapsed state. Thought and tool posts in the
//! feed render as one-line summaries when collapsed and expand back when
//! toggled again.

use crate::ui::view;
use ratatui::{backend::TestBackend, Terminal};
use runie_core::event::ControlEvent;
use runie_core::{AppState, ChatMessage, Event, Role};

fn count_matching_lines(state: &AppState, markers: &[&str]) -> usize {
    let backend = TestBackend::new(40, 12);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state.clone())).unwrap();
    let buf = terminal.backend().buffer();
    (0..buf.area().height)
        .filter(|&y| {
            let line: String = (0..buf.area().width)
                .map(|x| buf[(x, y)].symbol())
                .collect();
            markers.iter().any(|m| line.contains(m))
        })
        .count()
}

#[test]
fn ctrl_shift_e_collapses_thought_post_in_feed() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();

    state.session.messages.push(ChatMessage {
        role: Role::User,
        content: "hi".to_string(),
        timestamp: 0.0,
        id: "u1".to_string(),
        ..Default::default()
    });
    state.session.messages.push(ChatMessage {
        role: Role::Thought,
        content: "line1\nline2\nline3".to_string(),
        timestamp: 1.0,
        id: "t1".to_string(),
        ..Default::default()
    });
    state.messages_changed();
    state.ensure_fresh();

    let before = count_matching_lines(&state, &["line1", "line2", "line3"]);
    assert!(
        before >= 2,
        "expanded thought post should show content lines, got {before}"
    );

    state.update(Event::Control(ControlEvent::ToggleExpand));
    state.ensure_fresh();

    let after = count_matching_lines(&state, &["line1", "line2", "line3"]);
    assert!(
        after < before,
        "Ctrl+O should collapse thought post lines: before={before}, after={after}"
    );
}

#[test]
fn ctrl_shift_e_collapses_tool_post_in_feed() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();

    state.session.messages.push(ChatMessage {
        role: Role::User,
        content: "list files".to_string(),
        timestamp: 0.0,
        id: "u1".to_string(),
        ..Default::default()
    });
    state.session.messages.push(ChatMessage {
        role: Role::Tool,
        content: "✓ list_dir 0.5s\nfile1.rs\nfile2.rs".to_string(),
        timestamp: 1.0,
        id: "tool.u1.1".to_string(),
        ..Default::default()
    });
    state.messages_changed();
    state.ensure_fresh();

    let before = count_matching_lines(&state, &["file1.rs", "file2.rs"]);
    assert!(
        before >= 2,
        "expanded tool post should show output lines, got {before}"
    );

    state.update(Event::Control(ControlEvent::ToggleExpand));
    state.ensure_fresh();

    let after = count_matching_lines(&state, &["file1.rs", "file2.rs"]);
    assert!(
        after < before,
        "Ctrl+O should collapse tool post output: before={before}, after={after}"
    );
}

#[test]
fn ctrl_shift_e_twice_restores_post_lines() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();

    state.session.messages.push(ChatMessage {
        role: Role::Thought,
        content: "alpha\nbeta".to_string(),
        timestamp: 0.0,
        id: "t1".to_string(),
        ..Default::default()
    });
    state.messages_changed();
    state.ensure_fresh();

    let original = count_matching_lines(&state, &["alpha", "beta"]);

    state.update(Event::Control(ControlEvent::ToggleExpand));
    state.ensure_fresh();
    let collapsed = count_matching_lines(&state, &["alpha", "beta"]);
    assert!(collapsed < original, "thought post should collapse");

    state.update(Event::Control(ControlEvent::ToggleExpand));
    state.ensure_fresh();
    let restored = count_matching_lines(&state, &["alpha", "beta"]);
    assert_eq!(
        restored, original,
        "second Ctrl+O should restore thought post lines"
    );
}
