//! Rendering tests for expand/collapse of feed posts.
//!
//! Ctrl+O toggles the global collapsed state of TOOL posts. Thought posts
//! render as one-line summaries by default (grok parity) and are expanded
//! individually with Enter in feed navigation (Esc, then Enter on the post).

use super::*;
use crate::ui::view;
use ratatui::{backend::TestBackend, Terminal};
use runie_core::Event;

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
fn enter_expands_thought_post_in_feed() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();

    state.session.messages.push(ChatMessage {
        role: Role::User,
        parts: vec![Part::Text { content: "hi".into() }],
        timestamp: 0.0,
        id: "u1".to_string(),
        ..Default::default()
    });
    state.session.messages.push(ChatMessage {
        role: Role::Thought,
        parts: vec![Part::Text { content: "line1\nline2\nline3".into() }],
        timestamp: 1.0,
        id: "t1".to_string(),
        ..Default::default()
    });
    state.refresh_after_message_change();

    // Thoughts are summarized by default: only the first line shows.
    let collapsed = count_matching_lines(&state, &["line1", "line2", "line3"]);
    assert!(
        collapsed <= 1,
        "thought post should be summarized by default, got {collapsed}"
    );

    // Per-item expansion: Esc enters feed nav and selects the bottom post
    // (the thought); Enter expands it.
    state.update(Event::DialogBack);
    assert!(state.view.vim_nav_mode, "Esc should enter feed navigation");
    state.update(Event::Submit);
    state.ensure_fresh();

    let expanded = count_matching_lines(&state, &["line1", "line2", "line3"]);
    assert!(
        expanded >= 2,
        "Enter should expand the thought post lines: collapsed={collapsed}, expanded={expanded}"
    );
}

#[test]
fn ctrl_shift_e_collapses_tool_post_in_feed() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();

    state.session.messages.push(ChatMessage {
        role: Role::User,
        parts: vec![Part::Text { content: "list files".into() }],
        timestamp: 0.0,
        id: "u1".to_string(),
        ..Default::default()
    });
    state.session.messages.push(ChatMessage {
        role: Role::Tool,
        parts: vec![Part::Text { content: "✓ list_dir 0.5s\nfile1.rs\nfile2.rs".into() }],
        timestamp: 1.0,
        id: "tool.u1.1".to_string(),
        ..Default::default()
    });
    state.refresh_after_message_change();

    let before = count_matching_lines(&state, &["file1.rs", "file2.rs"]);
    assert!(
        before >= 2,
        "expanded tool post should show output lines, got {before}"
    );

    state.update(Event::ToggleExpand);
    state.ensure_fresh();

    let after = count_matching_lines(&state, &["file1.rs", "file2.rs"]);
    assert!(
        after < before,
        "Ctrl+O should collapse tool post output: before={before}, after={after}"
    );
}

#[test]
fn enter_twice_restores_thought_summary() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();

    state.session.messages.push(ChatMessage {
        role: Role::Thought,
        parts: vec![Part::Text { content: "alpha\nbeta".into() }],
        timestamp: 0.0,
        id: "t1".to_string(),
        ..Default::default()
    });
    state.refresh_after_message_change();

    let summary = count_matching_lines(&state, &["alpha", "beta"]);

    // Esc enters feed nav (selects the thought, the only post); Enter
    // expands it, a second Enter collapses it back to the summary.
    state.update(Event::DialogBack);
    state.update(Event::Submit);
    state.ensure_fresh();
    let expanded = count_matching_lines(&state, &["alpha", "beta"]);
    assert!(expanded > summary, "Enter should expand the thought post");

    state.update(Event::Submit);
    state.ensure_fresh();
    let restored = count_matching_lines(&state, &["alpha", "beta"]);
    assert_eq!(
        restored, summary,
        "second Enter should collapse the thought back to its summary"
    );
}
