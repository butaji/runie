use crossterm::event::{Event, KeyCode, KeyModifiers};
use crate::tui::state::{AppState, TuiMode, Msg};

pub fn event_to_msg(event: Event, state: &AppState) -> Option<Msg> {
    match event {
        Event::Key(key) => key_to_msg(key, state),
        _ => None,
    }
}

pub fn key_to_msg(key: crossterm::event::KeyEvent, state: &AppState) -> Option<Msg> {
    // Global hotkeys: always active regardless of mode
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('c') | KeyCode::Char('q') => return Some(Msg::Quit),
            _ => {}
        }
    }

    match state.mode {
        TuiMode::Chat => key_to_chat_msg(key),
        TuiMode::Permission => key_to_permission_msg(key),
        TuiMode::CommandPalette => key_to_palette_msg(key),
        TuiMode::DiffViewer => key_to_diff_msg(key),
        TuiMode::SessionTree => key_to_tree_msg(key),
        TuiMode::Onboarding => key_to_onboarding_msg(key),
        _ => None,
    }
}

fn key_to_chat_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return ctrl_chat_key(key);
    }
    match key.code {
        KeyCode::Enter => Some(Msg::Submit),
        KeyCode::Char(c) => Some(Msg::InsertChar(c)),
        KeyCode::Backspace => Some(Msg::Backspace),
        KeyCode::Left => Some(Msg::MoveCursorLeft),
        KeyCode::Right => Some(Msg::MoveCursorRight),
        KeyCode::Up => Some(Msg::MoveCursorUp),
        KeyCode::Down => Some(Msg::MoveCursorDown),
        KeyCode::PageUp => Some(Msg::ScrollPageUp),
        KeyCode::PageDown => Some(Msg::ScrollPageDown),
        _ => None,
    }
}

fn ctrl_chat_key(key: crossterm::event::KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Char('j') => Some(Msg::InsertNewline),
        KeyCode::Char('k') | KeyCode::Char('p') => Some(Msg::OpenCommandPalette),
        KeyCode::Char('a') => Some(Msg::MoveCursorToStart),
        KeyCode::Char('e') => Some(Msg::MoveCursorToEnd),
        KeyCode::Char('w') => Some(Msg::DeleteWordBackward),
        KeyCode::Char('u') => Some(Msg::DeleteToStart),
        KeyCode::Char('d') => Some(Msg::DeleteForward),
        KeyCode::Char('b') => Some(Msg::ToggleSidebar),
        KeyCode::Char('f') => Some(Msg::MoveCursorRight),
        KeyCode::Char('n') => Some(Msg::MoveCursorDown),
        KeyCode::Char('h') => Some(Msg::Backspace),
        KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => Some(Msg::InsertNewline),
        _ => None,
    }
}

fn key_to_permission_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Enter | KeyCode::Char('y') => Some(Msg::PermissionConfirm),
        KeyCode::Esc | KeyCode::Char('n') => Some(Msg::PermissionCancel),
        KeyCode::Char('a') => Some(Msg::PermissionAlways),
        KeyCode::Char('s') => Some(Msg::PermissionSkip),
        _ => None,
    }
}

fn key_to_palette_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Esc => Some(Msg::CloseModal),
        KeyCode::Enter => Some(Msg::CommandPaletteConfirm),
        KeyCode::Up => Some(Msg::CommandPaletteUp),
        KeyCode::Down => Some(Msg::CommandPaletteDown),
        KeyCode::Backspace => Some(Msg::CommandPaletteBackspace),
        KeyCode::Char(c) => Some(Msg::CommandPaletteFilter(c)),
        _ => None,
    }
}

fn key_to_diff_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => Some(Msg::CloseModal),
        KeyCode::Down | KeyCode::Char('j') => Some(Msg::ScrollDown),
        KeyCode::Up | KeyCode::Char('k') => Some(Msg::ScrollUp),
        KeyCode::PageDown => Some(Msg::ScrollDown),
        KeyCode::PageUp => Some(Msg::ScrollUp),
        _ => None,
    }
}

fn key_to_tree_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Esc => Some(Msg::CloseModal),
        KeyCode::Up | KeyCode::Char('k') => Some(Msg::SessionTreeUp),
        KeyCode::Down | KeyCode::Char('j') => Some(Msg::SessionTreeDown),
        KeyCode::Enter => Some(Msg::SessionTreeConfirm),
        _ => None,
    }
}

fn key_to_onboarding_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Enter => Some(Msg::OnboardingNext),
        KeyCode::Esc => Some(Msg::OnboardingBack),
        KeyCode::Up => Some(Msg::OnboardingBack),
        KeyCode::Down => Some(Msg::OnboardingNext),
        KeyCode::Char(c) => Some(Msg::OnboardingKeyInput(c)),
        KeyCode::Backspace => Some(Msg::OnboardingKeyBackspace),
        _ => None,
    }
}
