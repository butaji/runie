//! Tests for InputReceiver: Esc closes dialogs without triggering vim-nav.
//!
//! The `InputReceiver` enum tracks which component is currently receiving
//! keyboard input. When a dialog is open, Esc should close it without
//! entering vim-nav mode. Only when the chat input is active should Esc
//! enter vim-nav mode.

use crate::commands::{DialogKind, DialogState};
use crate::dialog::{ItemAction, Panel, PanelStack};
use crate::model::{AppState, InputReceiver};

fn state_with_vim() -> AppState {
    let mut state = AppState::default();
    state.config.vim_mode = true;
    state
}

#[test]
fn input_receiver_is_chat_input_by_default() {
    let state = AppState::default();
    assert_eq!(
        state.view.input_receiver,
        InputReceiver::ChatInput,
        "input_receiver should default to ChatInput"
    );
}

#[test]
fn opening_dialog_sets_input_receiver_to_dialog() {
    let mut state = state_with_vim();
    assert_eq!(
        state.view.input_receiver,
        InputReceiver::ChatInput,
        "input_receiver starts as ChatInput"
    );

    // Open command palette via input event
    state.update(crate::Event::Input('/'));
    assert_eq!(
        state.view.input_receiver,
        InputReceiver::Dialog,
        "opening command palette should set input_receiver to Dialog"
    );
}

#[test]
fn esc_closes_command_palette_without_triggering_vim_nav() {
    let mut state = state_with_vim();
    assert!(!state.view.vim_nav_mode);

    // Open command palette
    state.update(crate::Event::Input('/'));
    assert!(
        state.open_dialog.is_some(),
        "command palette should be open"
    );

    // Press Esc to close
    state.update(crate::Event::DialogBack);

    assert!(
        state.open_dialog.is_none(),
        "command palette should be closed"
    );
    assert!(
        !state.view.vim_nav_mode,
        "Esc closing command palette must NOT trigger vim-nav mode"
    );
    assert_eq!(
        state.view.input_receiver,
        InputReceiver::ChatInput,
        "input_receiver should be reset to ChatInput"
    );
}

#[test]
fn esc_after_closing_dialog_still_allows_vim_nav() {
    let mut state = state_with_vim();

    // Open and close command palette
    state.update(crate::Event::Input('/'));
    state.update(crate::Event::DialogBack);
    assert!(state.open_dialog.is_none());

    // Now Esc should enter vim-nav (since no dialog is open)
    state.update(crate::Event::DialogBack);
    assert!(
        state.view.vim_nav_mode,
        "Esc after closing dialog should enter vim-nav mode"
    );
}

#[test]
fn esc_closes_settings_dialog_without_triggering_vim_nav() {
    let mut state = state_with_vim();

    // Open settings dialog directly
    state.open_dialog = Some(DialogState::Active { kind: DialogKind::Settings, panels: PanelStack::new(
        Panel::new("settings", "Settings").item("Done", ItemAction::Close),
    ) });
    state.view.input_receiver = InputReceiver::Dialog;

    // Press Esc to close
    state.update(crate::Event::SettingsClose);

    assert!(
        state.open_dialog.is_none(),
        "settings dialog should be closed"
    );
    assert!(
        !state.view.vim_nav_mode,
        "Esc closing settings must NOT trigger vim-nav mode"
    );
}

#[test]
fn opening_model_selector_sets_input_receiver_to_dialog() {
    let mut state = state_with_vim();
    // Open model selector
    crate::update::dialog::open_model_selector(&mut state);
    assert_eq!(
        state.view.input_receiver,
        InputReceiver::Dialog,
        "opening model selector should set input_receiver to Dialog"
    );
}

#[test]
fn esc_closes_model_selector_without_triggering_vim_nav() {
    let mut state = state_with_vim();

    // Open model selector
    crate::update::dialog::open_model_selector(&mut state);
    assert!(state.open_dialog.is_some());

    // Press Esc to close
    state.update(crate::Event::DialogBack);

    assert!(
        state.open_dialog.is_none(),
        "model selector should be closed"
    );
    assert!(
        !state.view.vim_nav_mode,
        "Esc closing model selector must NOT trigger vim-nav mode"
    );
    assert_eq!(
        state.view.input_receiver,
        InputReceiver::ChatInput,
        "input_receiver should be reset to ChatInput"
    );
}
