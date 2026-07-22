//! Shared helpers for vim-nav rendering tests.

use super::*;
use crate::ui::view;
use ratatui::{backend::TestBackend, Terminal};
use runie_core::Event;

pub(crate) fn accent() -> ratatui::style::Color {
    crate::theme::color_accent()
}

pub(crate) fn accent_bg() -> ratatui::style::Color {
    crate::theme::color_accent_bg()
}

pub(crate) fn draw(state: &mut AppState, width: u16, height: u16) -> ratatui::buffer::Buffer {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, state)).unwrap();
    terminal.backend().buffer().clone()
}

pub(crate) fn bracket_rows(buf: &ratatui::buffer::Buffer) -> Vec<u16> {
    (0..buf.area().height)
        .filter(|&y| {
            let cell = &buf[(0, y)];
            cell.symbol() == "▎" && cell.style().fg == Some(accent())
        })
        .collect()
}

pub(crate) fn add_message(state: &mut AppState, role: Role, content: &str, timestamp: f64, id: &str) {
    state.session.messages.push(ChatMessage {
        role,
        parts: vec![Part::Text { content: content.to_string() }],
        timestamp,
        id: id.to_string(),
        ..Default::default()
    });
}

pub(crate) fn enter_vim_nav(state: &mut AppState) {
    state.config.vim_mode = true;
    state.refresh_after_message_change();

    state.update(Event::DialogBack);
    assert!(state.view.vim_nav_mode);
}

pub(crate) fn enter_vim_nav_and_select_top(state: &mut AppState) {
    enter_vim_nav(state);
    state.update(Event::Input('g'));
}

pub(crate) fn assert_bracket_one_cell_wide(buf: &ratatui::buffer::Buffer, rows: &[u16]) {
    for &y in rows {
        let next = &buf[(1, y)];
        assert!(
            next.symbol() != "▎",
            "bracket must not spill into the second column"
        );
    }
}

pub(crate) fn state_with_user_agent_pairs(count: usize) -> AppState {
    let mut state = AppState::default();
    state.config.vim_mode = true;
    for i in 0..count {
        add_message(
            &mut state,
            Role::User,
            &format!("message {}", i),
            i as f64,
            &format!("req.{}", i),
        );
        add_message(
            &mut state,
            Role::Assistant,
            &format!("reply {}", i),
            i as f64 + 0.5,
            &format!("resp.{}", i),
        );
    }
    state.refresh_after_message_change();

    state.view.last_visible_height = 10;
    state
}

pub(crate) fn state_with_wrapped_welcome() -> AppState {
    let mut state = AppState::default();
    state.config.vim_mode = true;
    state.session.messages.push(ChatMessage {
        role: Role::System,
        parts: vec![Part::Text {
            content: "Welcome to runie in someproject.\n\nThis project is not yet trusted. \
                  Run /trust to enable write tools, or /untrust to enforce read-only mode."
                .to_string(),
        }],
        timestamp: 0.0,
        id: "trust_welcome".to_string(),
        ..Default::default()
    });
    add_message(&mut state, Role::User, "hi", 1.0, "req.0");
    state.refresh_after_message_change();

    state.view.last_visible_height = 12;
    enter_vim_nav(&mut state);
    state.update(Event::Input('k'));
    assert_eq!(state.view.selected_post, Some(0));
    state
}

pub(crate) fn state_with_selected_post() -> AppState {
    let mut state = AppState::default();
    state.config.vim_mode = true;
    state.session.messages.push(ChatMessage {
        role: Role::User,
        parts: vec![Part::Text { content: "hello".into() }],
        timestamp: 0.0,
        id: "req.0".to_string(),
        ..Default::default()
    });
    state.session.messages.push(ChatMessage {
        role: Role::Assistant,
        parts: vec![Part::Text { content: "world".into() }],
        timestamp: 1.0,
        id: "resp.0".to_string(),
        provider: "mock".to_string(),
        ..Default::default()
    });
    state.refresh_after_message_change();

    enter_vim_nav(&mut state);
    state
}
