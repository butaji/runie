//! Rendering tests for global expand/collapse (Ctrl+Shift+E).

use crate::ui::view;
use ratatui::{backend::TestBackend, Terminal};
use runie_core::{AppState, ChatMessage, Event, Role};

#[test]
fn toggle_expand_collapses_thoughts_in_feed() {
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

    let count_expanded = |state: &AppState| {
        let backend = TestBackend::new(40, 12);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| view(f, &mut state.clone())).unwrap();
        let buf = terminal.backend().buffer();
        (0..buf.area().height)
            .filter(|&y| {
                let line: String = (0..buf.area().width)
                    .map(|x| buf[(x, y)].symbol())
                    .collect();
                line.contains("line1") || line.contains("line2") || line.contains("line3")
            })
            .count()
    };

    let before = count_expanded(&state);
    assert!(
        before >= 2,
        "expanded thought should show content lines, got {before}"
    );

    state.update(Event::ToggleExpand);
    state.ensure_fresh();

    let after = count_expanded(&state);
    assert!(
        after < before,
        "ToggleExpand should collapse thought lines: before={before}, after={after}"
    );
}

#[test]
fn toggle_expand_twice_restores_thought_lines() {
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

    let count_lines = |state: &AppState| {
        let backend = TestBackend::new(40, 12);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| view(f, &mut state.clone())).unwrap();
        let buf = terminal.backend().buffer();
        (0..buf.area().height)
            .filter(|&y| {
                let line: String = (0..buf.area().width)
                    .map(|x| buf[(x, y)].symbol())
                    .collect();
                line.contains("alpha") || line.contains("beta")
            })
            .count()
    };

    let original = count_lines(&state);
    state.update(Event::ToggleExpand);
    state.ensure_fresh();
    let collapsed = count_lines(&state);
    assert!(collapsed < original, "thought should collapse");

    state.update(Event::ToggleExpand);
    state.ensure_fresh();
    let restored = count_lines(&state);
    assert_eq!(
        restored, original,
        "second ToggleExpand should restore thought lines"
    );
}
