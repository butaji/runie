use crate::ui::view;
use ratatui::{backend::TestBackend, Terminal};
use runie_core::AppState;
use runie_core::Part;
use runie_util::labels::format_timestamp;

#[test]
fn user_message_has_border_bg_color() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.session.messages.push(runie_core::ChatMessage {
        role: runie_core::Role::User,
        parts: vec![Part::Text {
            content: "hello".into(),
        }],
        timestamp: 12345.0,
        id: "msg.1".to_string(),
        ..Default::default()
    });
    state.messages_changed();
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let expected_bg = crate::theme::color_accent_bg();

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
        parts: vec![Part::Text {
            content: "line1\nline2\nline3".into(),
        }],
        timestamp: 12345.0,
        id: "msg.1".to_string(),
        ..Default::default()
    });
    state.messages_changed();
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();

    let ts_str = format_timestamp(12345.0);
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
        parts: vec![Part::Text {
            content: "short".into(),
        }],
        timestamp: 12345.0,
        id: "msg.1".to_string(),
        ..Default::default()
    });
    state.messages_changed();
    let backend = TestBackend::new(80, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();

    let ts_str = format_timestamp(12345.0);
    let mut found = false;
    for y in 0..buf.area().height {
        let line: String = (0..buf.area().width)
            .map(|x| buf[(x, y)].symbol())
            .collect();
        if line.contains(&ts_str) {
            let pos = line.find(&ts_str).unwrap();
            assert!(
                pos > 60,
                "Timestamp should be right-aligned, got pos {}",
                pos
            );
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
        parts: vec![Part::Text {
            content: "first line\nsecond line".into(),
        }],
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

    let ts_str = format_timestamp(99999.0);
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
fn timestamp_never_wraps_even_when_content_is_very_long() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    let long_content = "a".repeat(200);
    state.session.messages.push(runie_core::ChatMessage {
        role: runie_core::Role::User,
        parts: vec![Part::Text {
            content: long_content,
        }],
        timestamp: 12345.0,
        id: "msg.1".to_string(),
        ..Default::default()
    });
    state.messages_changed();
    // Scroll to the top of the feed so the first (timestamp) line of the
    // wrapped message is visible. The test asserts the timestamp never
    // wraps to a second line, not that it is always in the bottom viewport.
    state.view.scroll = usize::MAX;
    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();

    let ts_str = format_timestamp(12345.0);
    let mut found_y: Option<u16> = None;
    for y in 0..buf.area().height {
        let line: String = (0..buf.area().width)
            .map(|x| buf[(x, y)].symbol())
            .collect();
        if line.contains(&ts_str) {
            if let Some(prev_y) = found_y {
                panic!(
                    "Timestamp wrapped! Found on line {} and again on line {}",
                    prev_y, y
                );
            }
            found_y = Some(y);
        }
    }
    assert!(found_y.is_some(), "Timestamp should be visible somewhere");
}
