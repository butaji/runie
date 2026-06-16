//! Crossterm key event → CoreEvent conversion with configurable keybindings.

use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use runie_core::event::{ControlEvent, DialogEvent, InputEvent};
use runie_core::{keybindings, Event as CoreEvent};
use std::collections::HashMap;

pub fn convert_event(event: &Event, bindings: &HashMap<String, String>) -> Option<CoreEvent> {
    log_key_event(event);
    match event {
        Event::Paste(data) => Some(CoreEvent::Input(InputEvent::Paste(data.clone()))),
        Event::Mouse(mouse) => convert_mouse_event(mouse),
        Event::FocusGained => Some(CoreEvent::Input(InputEvent::FocusGained)),
        Event::FocusLost => Some(CoreEvent::Input(InputEvent::FocusLost)),
        Event::Resize(width, height) => Some(CoreEvent::Input(InputEvent::TerminalSize {
            width: *width,
            height: *height,
        })),
        Event::Key(key) if is_press_or_repeat(key) => convert_key_event(key, bindings),
        _ => None,
    }
}

fn is_press_or_repeat(key: &KeyEvent) -> bool {
    key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat
}

fn convert_key_event(key: &KeyEvent, bindings: &HashMap<String, String>) -> Option<CoreEvent> {
    if key.modifiers.is_empty() && key.code == KeyCode::Char('\n') {
        return Some(CoreEvent::Input(InputEvent::Newline));
    }
    if key.modifiers.contains(KeyModifiers::SHIFT) && is_enter_like(key.code) {
        return Some(CoreEvent::Input(InputEvent::Newline));
    }
    if key.code == KeyCode::F(3) {
        return Some(CoreEvent::Input(InputEvent::Newline));
    }
    map_key_event(key, bindings)
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
    if let Some(evt) = lookup_binding(key, bindings) {
        return Some(evt);
    }
    map_by_modifier(key)
}

fn lookup_binding(key: &KeyEvent, bindings: &HashMap<String, String>) -> Option<CoreEvent> {
    let combo = key_event_to_combo(key);
    if combo.is_empty() {
        return None;
    }
    let event_name = bindings.get(&combo)?;
    keybindings::event_from_name(event_name)
}

fn map_by_modifier(key: &KeyEvent) -> Option<CoreEvent> {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
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
        KeyCode::Char('e') | KeyCode::Char('E') => Some(CoreEvent::Input(InputEvent::CursorEnd)),
        KeyCode::Char('o') | KeyCode::Char('O') => Some(CoreEvent::Control(ControlEvent::ToggleExpand)),
        KeyCode::Char('j') | KeyCode::Char('J') => Some(CoreEvent::Input(InputEvent::Newline)),
        KeyCode::Char('a') | KeyCode::Char('A') => Some(CoreEvent::Input(InputEvent::CursorStart)),
        KeyCode::Char('b') | KeyCode::Char('B') => Some(CoreEvent::Input(InputEvent::CursorLeft)),
        KeyCode::Char('f') | KeyCode::Char('F') => Some(CoreEvent::Input(InputEvent::CursorRight)),
        KeyCode::Char('w') | KeyCode::Char('W') => Some(CoreEvent::Input(InputEvent::DeleteWord)),
        KeyCode::Char('k') | KeyCode::Char('K') => Some(CoreEvent::Input(InputEvent::DeleteToEnd)),
        KeyCode::Char('u') | KeyCode::Char('U') => Some(CoreEvent::Input(InputEvent::DeleteToStart)),
        KeyCode::Char('d') | KeyCode::Char('D') => Some(CoreEvent::Input(InputEvent::KillChar)),
        KeyCode::Char('z') | KeyCode::Char('Z') => Some(CoreEvent::Control(ControlEvent::Suspend)),
        KeyCode::Char('y') | KeyCode::Char('Y') => Some(CoreEvent::Input(InputEvent::Redo)),
        KeyCode::Char('c') | KeyCode::Char('C') => Some(CoreEvent::Control(ControlEvent::Quit)),
        KeyCode::Char('q') | KeyCode::Char('Q') => Some(CoreEvent::Control(ControlEvent::Quit)),
        KeyCode::Char('s') | KeyCode::Char('S') => Some(CoreEvent::Control(ControlEvent::Abort)),
        KeyCode::Char('l') | KeyCode::Char('L') => Some(CoreEvent::Dialog(DialogEvent::ToggleModelSelector)),
        _ => None,
    }
}

fn map_alt_key(code: KeyCode) -> Option<CoreEvent> {
    match code {
        KeyCode::Enter => Some(CoreEvent::Control(ControlEvent::FollowUp)),
        KeyCode::Char('b') | KeyCode::Char('B') => Some(CoreEvent::Input(InputEvent::CursorWordLeft)),
        KeyCode::Char('f') | KeyCode::Char('F') => Some(CoreEvent::Input(InputEvent::CursorWordRight)),
        _ => None,
    }
}

fn map_shift_key(code: KeyCode) -> Option<CoreEvent> {
    match code {
        KeyCode::Enter => Some(CoreEvent::Input(InputEvent::Newline)),
        // Shift+F3 is what some terminals send for Shift+Enter (via \e[13;2~ escape sequence)
        KeyCode::F(3) => Some(CoreEvent::Input(InputEvent::Newline)),
        // Shift+symbol: pass through as regular input (crossterm already provides the shifted char)
        KeyCode::Char(c) => Some(CoreEvent::Input(InputEvent::Input(c))),
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
        KeyCode::Esc => Some(CoreEvent::Dialog(DialogEvent::DialogBack)),
        KeyCode::Char('\t') | KeyCode::Tab | KeyCode::BackTab => Some(CoreEvent::Input(InputEvent::Input('\t'))),
        KeyCode::Char(c) => Some(CoreEvent::Input(InputEvent::Input(c))),
        KeyCode::Backspace => Some(CoreEvent::Input(InputEvent::Backspace)),
        KeyCode::Enter => Some(CoreEvent::Input(InputEvent::Submit)),
        KeyCode::Up => Some(CoreEvent::Input(InputEvent::HistoryPrev)),
        KeyCode::Down => Some(CoreEvent::Input(InputEvent::HistoryNext)),
        KeyCode::Left => Some(CoreEvent::Input(InputEvent::CursorLeft)),
        KeyCode::Right => Some(CoreEvent::Input(InputEvent::CursorRight)),
        KeyCode::Home => Some(CoreEvent::Input(InputEvent::CursorStart)),
        KeyCode::End => Some(CoreEvent::Input(InputEvent::CursorEnd)),
        KeyCode::Delete => Some(CoreEvent::Input(InputEvent::KillChar)),
        _ => None,
    }
}

/// Convert mouse events to CoreEvent.
fn convert_mouse_event(mouse: &MouseEvent) -> Option<CoreEvent> {
    match mouse.kind {
        MouseEventKind::ScrollDown => Some(CoreEvent::Scroll(runie_core::event::ScrollEvent::Down)),
        MouseEventKind::ScrollUp => Some(CoreEvent::Scroll(runie_core::event::ScrollEvent::Up)),
        MouseEventKind::Down(btn) => Some(CoreEvent::Input(InputEvent::MouseClick {
            row: mouse.row,
            col: mouse.column,
            button: mouse_button_to_string(btn),
        })),
        MouseEventKind::Up(btn) => Some(CoreEvent::Input(InputEvent::MouseRelease {
            row: mouse.row,
            col: mouse.column,
            button: mouse_button_to_string(btn),
        })),
        MouseEventKind::Drag(btn) => Some(CoreEvent::Input(InputEvent::MouseDrag {
            row: mouse.row,
            col: mouse.column,
            button: mouse_button_to_string(btn),
        })),
        _ => None,
    }
}

fn mouse_button_to_string(button: MouseButton) -> String {
    match button {
        MouseButton::Left => "left".to_string(),
        MouseButton::Right => "right".to_string(),
        MouseButton::Middle => "middle".to_string(),
    }
}

#[cfg(test)]
mod tests;
