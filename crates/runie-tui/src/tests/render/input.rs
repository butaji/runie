use super::*;
use crate::tests::connect_model;
use crate::ui::view;
use ratatui::{backend::TestBackend, Terminal};
use runie_core::Event;

#[test]
fn input_chevron_is_orange_when_token_held() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    connect_model(&mut state);
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let orange = crate::theme::color_accent();
    let mut found = false;
    for y in 0..buf.area().height {
        for x in 0..buf.area().width.saturating_sub(2) {
            if buf[(x, y)].symbol() == "❯" {
                assert_eq!(buf[(x, y)].style().fg, Some(orange));
                found = true;
            }
        }
    }
    assert!(found);
}

#[test]
fn input_chevron_is_gray_when_token_released() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    connect_model(&mut state);
    state.update(Event::ToggleCommandPalette);
    let backend = TestBackend::new(60, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let dim = crate::theme::color_dim();
    let mut found = false;
    for y in (0..buf.area().height).rev() {
        for x in 0..buf.area().width.saturating_sub(2) {
            if buf[(x, y)].symbol() == "❯" {
                assert_eq!(buf[(x, y)].style().fg, Some(dim));
                found = true;
                break;
            }
        }
        if found {
            break;
        }
    }
    assert!(found);
}

#[test]
fn palette_filter_uses_chevron_glyph() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    connect_model(&mut state);
    state.update(Event::ToggleCommandPalette);
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let content: String = flatten_buffer(buf);
    assert!(content.contains("❯"));
    assert!(!content.contains("> "));
}

#[test]
fn model_selector_filter_uses_chevron_glyph() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    connect_model(&mut state);
    state.update(Event::ToggleModelSelector);
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let content: String = flatten_buffer(buf);
    assert!(content.contains("❯"));
    assert!(!content.contains("> "));
}

#[test]
fn app_background_is_theme_bg_color() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    connect_model(&mut state);
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let expected_bg = crate::theme::color_bg();
    assert_eq!(buf[(0, 0)].style().bg, Some(expected_bg));
}

#[test]
fn input_cursor_visible_when_empty() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    connect_model(&mut state);
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let orange = crate::theme::color_accent();
    let mut found = false;
    for y in 0..buf.area().height {
        for x in 0..buf.area().width.saturating_sub(2) {
            if buf[(x, y)].symbol() == "❯" && buf[(x + 2, y)].style().bg == Some(orange) {
                found = true;
            }
        }
    }
    assert!(found);
}

#[test]
fn input_cursor_hidden_when_token_released() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    connect_model(&mut state);
    state.update(Event::ToggleCommandPalette);
    state.input.input = "hello".to_string();
    state.input.cursor_pos = 2;
    let backend = TestBackend::new(60, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let orange = crate::theme::color_accent();
    let mut found = false;
    for y in 0..buf.area().height {
        for x in 0..buf.area().width.saturating_sub(4) {
            let prefix: String = (x..x + 4).map(|cx| buf[(cx, y)].symbol()).collect();
            if prefix == "❯ he" && buf[(x + 4, y)].style().bg == Some(orange) {
                found = true;
            }
        }
    }
    assert!(!found);
}

#[test]
fn input_cursor_is_orange_when_token_held() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    connect_model(&mut state);
    state.input.input = "hello".to_string();
    state.input.cursor_pos = 2;
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let orange = crate::theme::color_accent();
    let mut found = false;
    for y in 0..buf.area().height {
        for x in 0..buf.area().width.saturating_sub(4) {
            let prefix: String = (x..x + 4).map(|cx| buf[(cx, y)].symbol()).collect();
            if prefix == "❯ he" {
                assert_eq!(buf[(x + 4, y)].style().bg, Some(orange));
                found = true;
                break;
            }
        }
        if found {
            break;
        }
    }
    assert!(found);
}

pub(crate) fn flatten_buffer(buf: &ratatui::buffer::Buffer) -> String {
    (0..buf.area().height)
        .map(|y| {
            (0..buf.area().width)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
        })
        .collect()
}
