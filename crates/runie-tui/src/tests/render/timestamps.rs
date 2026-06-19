use crate::ui::view;
use ratatui::{backend::TestBackend, Terminal};
use runie_core::AppState;

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
    let expected_bg = crate::theme::color_user_bg();

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
fn feed_messages_do_not_show_timestamps() {
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
    assert_eq!(count, 0, "Grok-style feed hides message timestamps");
}

#[test]
fn agent_message_does_not_show_timestamp() {
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
    assert_eq!(count, 0, "Agent messages should not show timestamps");
}
