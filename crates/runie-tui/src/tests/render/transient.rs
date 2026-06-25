use super::input::flatten_buffer;
use super::*;
use crate::ui::view;
use ratatui::{backend::TestBackend, Terminal};
use runie_core::AppState;

#[test]
fn transient_success_renders_green_background_with_ok_prefix() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState {
        transient_message: Some("Theme switched".to_string()),
        transient_level: Some(runie_core::event::TransientLevel::Success),
        ..Default::default()
    };
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let content = flatten_buffer(buf);
    assert!(content.contains("\\ok\\ "));
    assert!(content.contains("Theme switched"));
    assert!(!content.contains("ctrl+o"));

    let green = crate::theme::color_success();
    let mut found = false;
    for y in 0..buf.area().height {
        for x in 0..buf.area().width {
            if buf[(x, y)].symbol() == "T" && buf[(x, y)].style().bg == Some(green) {
                found = true;
            }
        }
    }
    assert!(found);
}

#[test]
fn transient_success_has_1_symbol_margin_on_both_sides() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState {
        transient_message: Some("Test".to_string()),
        transient_level: Some(runie_core::event::TransientLevel::Success),
        ..Default::default()
    };
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();

    let green = crate::theme::color_success();
    let margin_green = crate::theme::darken(green, 0.85);
    let badge_bg = crate::theme::darken(green, 0.8);

    let transient_y = (0..buf.area().height)
        .find(|&y| {
            (0..buf.area().width)
                .any(|x| buf[(x, y)].symbol() == "T" && buf[(x, y)].style().bg == Some(green))
        })
        .expect("Should find transient row");

    assert_eq!(buf[(1, transient_y)].style().bg, Some(margin_green));

    let last = buf.area().width - 2;
    for x in 2..=last {
        let bg = buf[(x, transient_y)].style().bg;
        assert!(
            bg == Some(green) || bg == Some(badge_bg) || bg == Some(margin_green),
            "Column {} should have valid bg",
            x
        );
    }
}

#[test]
fn transient_warning_renders_amber_background_with_warn_prefix() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState {
        transient_message: Some("Read-only on".to_string()),
        transient_level: Some(runie_core::event::TransientLevel::Warning),
        ..Default::default()
    };
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let content = flatten_buffer(buf);
    assert!(content.contains("\\warn\\ "));
    assert!(content.contains("Read-only on"));
}

#[test]
fn transient_error_renders_red_background_with_error_prefix() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState {
        transient_message: Some("Failed".to_string()),
        transient_level: Some(runie_core::event::TransientLevel::Error),
        ..Default::default()
    };
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let content = flatten_buffer(buf);
    assert!(content.contains("\\err\\ "));
    assert!(content.contains("Failed"));
}

#[test]
fn transient_message_renders_in_hints_line() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState {
        transient_message: Some("Test message".to_string()),
        ..Default::default()
    };
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    assert!(flatten_buffer(buf).contains("Test message"));
}

#[test]
fn default_hints_render_when_no_transient() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    assert!(flatten_buffer(buf).contains("ctrl+o"));
}

#[test]
fn streaming_tail_renders_when_turn_active() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    // Add a message so there's content in the feed
    state.update(runie_core::event::Event::Response {
        id: "test.1".into(),
        content: "Hello world".into(),
    });
    // Add streaming tail
    state.update(runie_core::event::Event::ResponseDelta {
        id: "test.1".into(),
        content: " and more".into(),
    });
    // Set turn_active to show streaming cell
    state.agent.turn_active = true;

    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let content = flatten_buffer(buf);

    // The tail content should appear in the rendered output
    assert!(
        content.contains("Hello world and more") || content.contains("and more"),
        "streaming tail should appear: {}",
        content
    );
}
