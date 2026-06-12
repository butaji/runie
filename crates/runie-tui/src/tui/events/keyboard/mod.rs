//! Event keyboard handling.

mod handlers;

use crossterm::event::{KeyCode, KeyModifiers};
use crate::tui::state::{AppState, TuiMode, Msg};

pub fn key_to_msg(key: crossterm::event::KeyEvent, state: &AppState) -> Option<Msg> {
    // P0-3/P0-4 FIX: Blocking modes intercept keys, but if a handler exists and
    // returns None (didn't handle), fall through to global_hotkey_handler
    if let Some(blocking_result) = blocking_mode_handler(&key, &state.mode, state) {
        if let Some(msg) = blocking_result {
            return Some(msg);
        }
        // blocking_result is None - handler existed but didn't handle this key
        // Fall through to global_hotkey_handler
    }
    
    // Global hotkeys: active in all non-blocking modes
    if let Some(global_result) = global_hotkey_handler(&key, state) {
        if let Some(msg) = global_result {
            return Some(msg);
        }
        // global_result is None - handler existed but didn't handle, fall through
    }

    // Route to mode-specific handler (non-blocking modes only)
    route_non_blocking_mode(&key, state)
}

/// Handles keys in blocking modes (Permission, Overlay).
fn blocking_mode_handler(key: &crossterm::event::KeyEvent, mode: &TuiMode, state: &AppState) -> Option<Option<Msg>> {
    use handlers::*;
    match mode {
        TuiMode::Permission => Some(key_to_permission_msg(*key)),
        TuiMode::Overlay => Some(key_to_overlay_msg(*key, state)),
        TuiMode::HomeScreen => Some(key_to_home_screen_msg(*key)),
        TuiMode::Plan => Some(key_to_plan_modal_msg(*key, state)),
        _ => None,
    }
}

/// Handles global hotkeys (Ctrl+C, Ctrl+Q) in non-blocking modes.
fn global_hotkey_handler(key: &crossterm::event::KeyEvent, state: &AppState) -> Option<Option<Msg>> {
    if !key.modifiers.contains(KeyModifiers::CONTROL) {
        return None;
    }
    // Ctrl+Shift+Q toggles questionnaire panel
    if is_ctrl_shift_q(key) {
        return Some(Some(Msg::ToggleQuestionnaire));
    }
    // DiffViewer intercepts Ctrl+Q to close the viewer
    if is_diffviewer_ctrl_q(key, state) {
        return None;
    }
    ctrl_hotkey_match(key, state)
}

fn is_ctrl_shift_q(key: &crossterm::event::KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::SHIFT) && matches!(key.code, KeyCode::Char('q'))
}

fn is_diffviewer_ctrl_q(key: &crossterm::event::KeyEvent, state: &AppState) -> bool {
    matches!(state.mode, TuiMode::DiffViewer) && matches!(key.code, KeyCode::Char('q'))
}

fn ctrl_hotkey_match(key: &crossterm::event::KeyEvent, state: &AppState) -> Option<Option<Msg>> {
    match key.code {
        KeyCode::Char('c') => ctrl_c_handler(state),
        KeyCode::Char('q') | KeyCode::Char('d') => ctrl_q_handler(),
        KeyCode::Char('m') => Some(Some(Msg::SwitchModel)),
        KeyCode::Char('h') => Some(Some(Msg::GoHome)),
        _ => None,
    }
}

fn ctrl_c_handler(state: &AppState) -> Option<Option<Msg>> {
    if state.agent_running {
        Some(Some(Msg::Stop))
    } else if state.textarea.lines() == [""] {
        Some(Some(Msg::Quit))
    } else {
        Some(Some(Msg::ClearInputConfirm))
    }
}

fn ctrl_q_handler() -> Option<Option<Msg>> {
    Some(Some(Msg::Quit))
}

/// Routes key to the appropriate mode-specific handler (non-blocking modes only).
fn route_non_blocking_mode(key: &crossterm::event::KeyEvent, state: &AppState) -> Option<Msg> {
    
    if let Some(msg) = check_modal_precedence(key, state) {
        return Some(msg);
    }
    if is_interject(key, state) {
        return Some(Msg::Interject);
    }
    route_by_mode(key, state)
}

fn is_interject(key: &crossterm::event::KeyEvent, state: &AppState) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Enter) && state.agent_running
}

fn check_modal_precedence(key: &crossterm::event::KeyEvent, state: &AppState) -> Option<Msg> {
    use handlers::*;
    if matches!(state.mode, TuiMode::Chat) && state.slash_menu.is_open() {
        return key_to_slash_menu_msg(*key);
    }
    if state.shortcuts_panel.is_open() {
        return key_to_shortcuts_panel_msg(*key, state);
    }
    if state.settings_modal.is_open() {
        return key_to_settings_modal_msg(*key);
    }
    if state.file_picker.is_open() {
        return key_to_file_picker_msg(*key);
    }
    if !state.history_search_matches.is_empty() {
        return key_to_history_search_msg(*key);
    }
    if state.context_usage_modal.is_open() {
        return key_to_context_usage_msg(*key);
    }
    None
}

fn route_by_mode(key: &crossterm::event::KeyEvent, state: &AppState) -> Option<Msg> {
    use handlers::*;
    match state.mode {
        TuiMode::Chat | TuiMode::Select => key_to_chat_msg(*key, state),
        TuiMode::CommandPalette => key_to_palette_msg(*key),
        TuiMode::DiffViewer => key_to_diff_msg(*key),
        TuiMode::SessionTree => key_to_tree_msg(*key),
        TuiMode::Onboarding => key_to_onboarding_msg(*key, state),
        TuiMode::Questionnaire => key_to_questionnaire_msg(*key),
        _ => {
            tracing::warn!("Unhandled TuiMode in route_non_blocking_mode");
            None
        }
    }
}
