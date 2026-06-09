use runie_core::{AppState, Event};
use crate::ui::view;
use ratatui::{backend::TestBackend, Terminal};

#[test]
fn user_message_has_border_bg_color() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.session.messages.push(runie_core::ChatMessage {
        role: runie_core::Role::User,
        content: "hello".to_string(),
        timestamp: 12345.0,
        id: "msg.1".to_string(),
        ..Default::default()
    });
    state.messages_changed();
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let expected_bg = crate::theme::darken(crate::theme::color_border(), 0.5);

    let mut found = false;
    for y in 0..buf.area().height {
        for x in 0..buf.area().width {
            if buf[(x, y)].symbol() == "h" && buf[(x, y)].style().bg == Some(expected_bg) {
                found = true;
            }
        }
    }
    assert!(found);
}

#[test]
fn timestamp_shown_once_per_message_element() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.session.messages.push(runie_core::ChatMessage {
        role: runie_core::Role::User,
        content: "line1\nline2\nline3".to_string(),
        timestamp: 12345.0,
        id: "msg.1".to_string(),
        ..Default::default()
    });
    state.messages_changed();
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();

    let ts_str = runie_core::format_timestamp(12345.0);
    let count = (0..buf.area().height)
        .filter(|y| {
            let line: String = (0..buf.area().width)
                .map(|x| buf[(x, *y)].symbol())
                .collect();
            line.contains(&ts_str)
        })
        .count();
    assert_eq!(count, 1);
}

#[test]
fn timestamp_right_aligned_on_first_line() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.session.messages.push(runie_core::ChatMessage {
        role: runie_core::Role::User,
        content: "short".to_string(),
        timestamp: 12345.0,
        id: "msg.1".to_string(),
        ..Default::default()
    });
    state.messages_changed();
    let backend = TestBackend::new(80, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();

    let ts_str = runie_core::format_timestamp(12345.0);
    let mut found = false;
    for y in 0..buf.area().height {
        let line: String = (0..buf.area().width)
            .map(|x| buf[(x, y)].symbol())
            .collect();
        if line.contains(&ts_str) {
            let pos = line.find(&ts_str).unwrap();
            assert!(pos > 60, "Timestamp should be right-aligned, got pos {}", pos);
            found = true;
        }
    }
    assert!(found);
}

#[test]
fn agent_message_timestamp_appears_once() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.session.messages.push(runie_core::ChatMessage {
        role: runie_core::Role::Assistant,
        content: "first line\nsecond line".to_string(),
        timestamp: 99999.0,
        id: "msg.1".to_string(),
        provider: "mock".to_string(),
        ..Default::default()
    });
    state.messages_changed();
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();

    let ts_str = runie_core::format_timestamp(99999.0);
    let count = (0..buf.area().height)
        .filter(|y| {
            let line: String = (0..buf.area().width)
                .map(|x| buf[(x, *y)].symbol())
                .collect();
            line.contains(&ts_str)
        })
        .count();
    assert_eq!(count, 1);
}
