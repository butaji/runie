use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use runie_core::Event as CoreEvent;
use std::collections::HashMap;

fn map_key_event(key: &KeyEvent, bindings: &HashMap<String, String>) -> Option<CoreEvent> {
    let combo = key_event_to_combo(key);
    if !combo.is_empty() {
        if let Some(event_name) = bindings.get(&combo) {
            if let Some(evt) = keybindings::event_from_name(event_name) {
                return Some(evt);
            }
        }
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        map_ctrl_key(key.code)
    } else if key.modifiers.contains(KeyModifiers::ALT) {
        map_alt_key(key.code)
    } else if key.modifiers.contains(KeyModifiers::SHIFT) {
        map_shift_key(key.code)
    } else {
        map_plain_key(key.code)
    }
}

fn map_ctrl_key(code: KeyCode) -> Option<CoreEvent> {
    match code {
        KeyCode::Char('e') | KeyCode::Char('E') => Some(CoreEvent::ToggleExpand),
        KeyCode::Char('j') | KeyCode::Char('J') => Some(CoreEvent::Newline),
        KeyCode::Char('a') | KeyCode::Char('A') => Some(CoreEvent::CursorStart),
        KeyCode::Char('b') | KeyCode::Char('B') => Some(CoreEvent::CursorLeft),
        KeyCode::Char('f') | KeyCode::Char('F') => Some(CoreEvent::CursorRight),
        KeyCode::Char('w') | KeyCode::Char('W') => Some(CoreEvent::DeleteWord),
        KeyCode::Char('k') | KeyCode::Char('K') => Some(CoreEvent::DeleteToEnd),
        KeyCode::Char('u') | KeyCode::Char('U') => Some(CoreEvent::DeleteToStart),
        KeyCode::Char('d') | KeyCode::Char('D') => Some(CoreEvent::KillChar),
        KeyCode::Char('z') | KeyCode::Char('Z') => Some(CoreEvent::Suspend),
        KeyCode::Char('y') | KeyCode::Char('Y') => Some(CoreEvent::Redo),
        KeyCode::Char('c') | KeyCode::Char('C') => Some(CoreEvent::Quit),
        KeyCode::Char('s') | KeyCode::Char('S') => Some(CoreEvent::Abort),
        KeyCode::Char('l') | KeyCode::Char('L') => Some(CoreEvent::ToggleModelSelector),
        _ => None,
    }
}

fn map_alt_key(code: KeyCode) -> Option<CoreEvent> {
    match code {
        KeyCode::Enter => Some(CoreEvent::FollowUp),
        KeyCode::Char('b') | KeyCode::Char('B') => Some(CoreEvent::CursorWordLeft),
        KeyCode::Char('f') | KeyCode::Char('F') => Some(CoreEvent::CursorWordRight),
        _ => None,
    }
}

fn map_shift_key(code: KeyCode) -> Option<CoreEvent> {
    match code {
        KeyCode::Enter => Some(CoreEvent::Newline),
        // Shift+F3 is what some terminals send for Shift+Enter (via \e[13;2~ escape sequence)
        KeyCode::F(3) => Some(CoreEvent::Newline),
        // Shift+symbol: pass through as regular input (crossterm already provides the shifted char)
        KeyCode::Char(c) => Some(CoreEvent::Input(c)),
        _ => None,
    }
}

fn map_plain_key(code: KeyCode) -> Option<CoreEvent> {
    match code {
        KeyCode::Esc => Some(CoreEvent::Abort),
        KeyCode::Char('\t') | KeyCode::Tab | KeyCode::BackTab => Some(CoreEvent::Input('\t')),
        KeyCode::Char(c) => Some(CoreEvent::Input(c)),
        KeyCode::Backspace => Some(CoreEvent::Backspace),
        KeyCode::Enter => Some(CoreEvent::Submit),
        KeyCode::Up => Some(CoreEvent::HistoryPrev),
        KeyCode::Down => Some(CoreEvent::HistoryNext),
        KeyCode::Left => Some(CoreEvent::CursorLeft),
        KeyCode::Right => Some(CoreEvent::CursorRight),
        KeyCode::Home => Some(CoreEvent::CursorStart),
        KeyCode::End => Some(CoreEvent::CursorEnd),
        KeyCode::Delete => Some(CoreEvent::KillChar),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
