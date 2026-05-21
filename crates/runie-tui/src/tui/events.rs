use crossterm::event::{Event, KeyCode, KeyModifiers};
use crate::tui::state::{AppState, TuiMode, Msg};

pub fn event_to_msg(event: Event, state: &AppState) -> Option<Msg> {
    match event {
        Event::Key(key) => key_to_msg(key, state),
        _ => None,
    }
}

pub fn key_to_msg(key: crossterm::event::KeyEvent, state: &AppState) -> Option<Msg> {
    match state.mode {
        TuiMode::Chat => match key.code {
            KeyCode::Char('c') | KeyCode::Char('q')
                if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Msg::Quit),
            KeyCode::Enter => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    Some(Msg::InsertNewline)
                } else {
                    Some(Msg::Submit)
                }
            }
            KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Msg::InsertNewline),
            KeyCode::Char('k') | KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Msg::OpenCommandPalette),
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Msg::MoveCursorToStart),
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Msg::MoveCursorToEnd),
            KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Msg::DeleteWordBackward),
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Msg::DeleteToStart),
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Msg::DeleteForward),
            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Msg::ToggleSidebar),
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Msg::MoveCursorRight),
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Msg::MoveCursorDown),
            KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Msg::Backspace),
            KeyCode::Char(c) => Some(Msg::InsertChar(c)),
            KeyCode::Backspace => Some(Msg::Backspace),
            KeyCode::Left => Some(Msg::MoveCursorLeft),
            KeyCode::Right => Some(Msg::MoveCursorRight),
            KeyCode::Up => Some(Msg::MoveCursorUp),
            KeyCode::Down => Some(Msg::MoveCursorDown),
            KeyCode::PageUp => Some(Msg::ScrollUp),
            KeyCode::PageDown => Some(Msg::ScrollDown),
            _ => None,
        },
        TuiMode::Permission => match key.code {
            KeyCode::Enter => Some(Msg::PermissionConfirm),
            KeyCode::Esc => Some(Msg::PermissionCancel),
            KeyCode::Char('y') => Some(Msg::PermissionConfirm),
            KeyCode::Char('n') => Some(Msg::PermissionCancel),
            KeyCode::Char('a') => Some(Msg::PermissionAlways),
            KeyCode::Char('s') => Some(Msg::PermissionSkip),
            _ => None,
        },
        TuiMode::CommandPalette => match key.code {
            KeyCode::Esc => Some(Msg::CloseModal),
            KeyCode::Enter => Some(Msg::CommandPaletteConfirm),
            KeyCode::Up => Some(Msg::CommandPaletteUp),
            KeyCode::Down => Some(Msg::CommandPaletteDown),
            KeyCode::Backspace => Some(Msg::CommandPaletteBackspace),
            KeyCode::Char(c) => Some(Msg::CommandPaletteFilter(c)),
            _ => None,
        },
        _ => None,
    }
}

// ─── to_agent_messages ─────────────────────────────────────────────────────────
// Convert MessageItem list to AgentMessage list for spawning agent

