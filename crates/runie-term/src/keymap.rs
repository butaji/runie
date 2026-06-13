//! Crossterm key event → CoreEvent conversion with configurable keybindings.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use runie_core::{keybindings, Event as CoreEvent};
use std::collections::HashMap;

pub fn convert_event(event: &Event, bindings: &HashMap<String, String>) -> Option<CoreEvent> {
    log_key_event(event);
    match event {
        Event::Paste(data) => Some(CoreEvent::Paste(data.clone())),
        Event::Key(key) if key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat => {
            // Handle Ctrl+J - ASCII 10 (LF) is often sent as just a char
            if let KeyCode::Char(c) = key.code {
                if key.modifiers.is_empty() && c == '\n' {
                    return Some(CoreEvent::Newline);
                }
            }
            // Broad Shift+Enter detection for various terminals (tmux, Warp, iTerm, etc.)
            if key.modifiers.contains(KeyModifiers::SHIFT) && is_enter_like(key.code) {
                return Some(CoreEvent::Newline);
            }
            // tmux sends \e[13;2~ for Shift+Enter — some crossterm versions report this as
            // F(3) without the SHIFT modifier bit set. Always treat F(3) as Newline.
            if key.code == KeyCode::F(3) {
                return Some(CoreEvent::Newline);
            }
            map_key_event(key, bindings)
        }
        _ => None,
    }
}

fn is_enter_like(code: KeyCode) -> bool {
    matches!(
        code,
        KeyCode::Enter
        | KeyCode::F(3)      // tmux sends \e[13;2~ for Shift+Enter
        | KeyCode::F(13)     // some terminals use F13
        | KeyCode::Char('\n')
        | KeyCode::Char('\r')
    )
}

fn log_key_event(event: &Event) {
    if let Event::Key(key) = event {
        if std::env::var("RUNIE_DEBUG").is_ok() {
            use std::io::Write;
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/runie_keys.log")
                .and_then(|mut f| writeln!(f, "{:?}", key));
        }
    }
}

/// Handle escape sequences that crossterm doesn't parse as KeyEvent.
/// Many terminals send different sequences for modified keys.
pub fn key_event_to_combo(key: &KeyEvent) -> String {
    let mut parts = Vec::new();
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        parts.push("ctrl");
    }
    if key.modifiers.contains(KeyModifiers::ALT) {
        parts.push("alt");
    }
    if key.modifiers.contains(KeyModifiers::SHIFT) {
        parts.push("shift");
    }
    let key_name = match key.code {
        KeyCode::Char('\t') => "tab".to_string(),
        KeyCode::Char(c) => c.to_lowercase().to_string(),
        KeyCode::Enter => "enter".to_string(),
        KeyCode::Esc => "escape".to_string(),
        KeyCode::Backspace => "backspace".to_string(),
        KeyCode::Tab => "tab".to_string(),
        KeyCode::BackTab => "shift+tab".to_string(),
        KeyCode::Delete => "delete".to_string(),
        KeyCode::Up => "up".to_string(),
        KeyCode::Down => "down".to_string(),
        KeyCode::Left => "left".to_string(),
        KeyCode::Right => "right".to_string(),
        KeyCode::Home => "home".to_string(),
        KeyCode::End => "end".to_string(),
        KeyCode::Insert => "insert".to_string(),
        KeyCode::PageUp => "pageup".to_string(),
        KeyCode::PageDown => "pagedown".to_string(),
        KeyCode::F(n) => format!("f{}", n),
        _ => return String::new(),
    };
    parts.push(&key_name);
    parts.join("+")
}

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
        // Ctrl+O collapses/expands feed posts. Ctrl+Shift+E is intentionally
        // unbound because many terminals (e.g. tmux without extended keys)
        // cannot distinguish it from Ctrl+E, which is cursor-end.
        if key.modifiers.contains(KeyModifiers::SHIFT)
            && matches!(key.code, KeyCode::Char('e') | KeyCode::Char('E'))
        {
            return None;
        }
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
        KeyCode::Char('e') | KeyCode::Char('E') => Some(CoreEvent::CursorEnd),
        KeyCode::Char('o') | KeyCode::Char('O') => Some(CoreEvent::ToggleExpand),
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
        KeyCode::Char('q') | KeyCode::Char('Q') => Some(CoreEvent::Quit),
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
        // Esc acts as a **Back button** in any open dialog (command bar,
        // settings, login flow, model selector, etc.). The dialog's
        // panel-stack handler interprets `DialogBack` as stack nav:
        // pop one panel when deeper, close the dialog when at the root
        // (the "main menu" of that bar). To force-close from any depth
        // use `Abort` (Ctrl+\) instead.
        KeyCode::Esc => Some(CoreEvent::DialogBack),
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
