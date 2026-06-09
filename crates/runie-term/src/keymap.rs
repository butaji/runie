//! Crossterm key event → CoreEvent conversion with configurable keybindings.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use runie_core::{Event as CoreEvent, keybindings};
use std::collections::HashMap;

pub fn convert_event(event: &Event, bindings: &HashMap<String, String>) -> Option<CoreEvent> {
    log_key_event(event);
    match event {
        Event::Paste(data) => Some(CoreEvent::Paste(data.clone())),
        Event::Key(key) if key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat => {
            map_key_event(key, bindings)
        }
        _ => None,
    }
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
        KeyCode::Tab | KeyCode::BackTab => "tab".to_string(),
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
        KeyCode::Char('z') | KeyCode::Char('Z') => Some(CoreEvent::Undo),
        KeyCode::Char('y') | KeyCode::Char('Y') => Some(CoreEvent::Redo),
        KeyCode::Char('c') | KeyCode::Char('C') => Some(CoreEvent::Quit),
        KeyCode::Char('s') | KeyCode::Char('S') => Some(CoreEvent::Abort),
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
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

    fn default_bindings() -> std::collections::HashMap<String, String> {
        runie_core::keybindings::default_keybindings()
    }

    #[test]
    fn ctrl_shift_e_converts_to_toggle_expand() {
        let key = KeyEvent::new(KeyCode::Char('E'), KeyModifiers::CONTROL | KeyModifiers::SHIFT);
        let event = crossterm::event::Event::Key(key);
        let result = super::convert_event(&event, &default_bindings());
        assert!(matches!(result, Some(runie_core::Event::ToggleExpand)), "Ctrl+Shift+E should map to ToggleExpand, got {:?}", result);
    }

    #[test]
    fn ctrl_e_converts_to_toggle_expand_for_terminals_without_shift() {
        let key = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL);
        let event = crossterm::event::Event::Key(key);
        let result = super::convert_event(&event, &default_bindings());
        assert!(matches!(result, Some(runie_core::Event::ToggleExpand)), "Ctrl+E should map to ToggleExpand, got {:?}", result);
    }

    #[test]
    fn ctrl_c_converts_to_quit() {
        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        let event = crossterm::event::Event::Key(key);
        let result = super::convert_event(&event, &default_bindings());
        assert!(matches!(result, Some(runie_core::Event::Quit)), "Ctrl+C should map to Quit");
    }

    #[test]
    fn plain_e_not_converted() {
        let key = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::empty());
        let event = crossterm::event::Event::Key(key);
        let result = super::convert_event(&event, &default_bindings());
        assert!(matches!(result, Some(runie_core::Event::Input('e'))), "Plain e should map to Input");
    }

    #[test]
    fn ctrl_e_does_not_conflict_with_quit() {
        let key = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL);
        let event = crossterm::event::Event::Key(key);
        let result = super::convert_event(&event, &default_bindings());
        assert!(!matches!(result, Some(runie_core::Event::Quit)), "Ctrl+E should NOT map to Quit");
    }

    #[test]
    fn ctrl_shift_e_on_repeat_kind_still_works() {
        let key = KeyEvent::new_with_kind(KeyCode::Char('E'), KeyModifiers::CONTROL | KeyModifiers::SHIFT, KeyEventKind::Repeat);
        let event = crossterm::event::Event::Key(key);
        let result = super::convert_event(&event, &default_bindings());
        assert!(matches!(result, Some(runie_core::Event::ToggleExpand)), "Ctrl+Shift+E with Repeat kind should still map to ToggleExpand, got {:?}", result);
    }

    #[test]
    fn ctrl_e_on_repeat_kind_still_works() {
        let key = KeyEvent::new_with_kind(KeyCode::Char('e'), KeyModifiers::CONTROL, KeyEventKind::Repeat);
        let event = crossterm::event::Event::Key(key);
        let result = super::convert_event(&event, &default_bindings());
        assert!(matches!(result, Some(runie_core::Event::ToggleExpand)), "Ctrl+E with Repeat kind should still map to ToggleExpand, got {:?}", result);
    }

    #[test]
    fn ctrl_z_converts_to_undo() {
        let key = KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL);
        let event = crossterm::event::Event::Key(key);
        let result = super::convert_event(&event, &default_bindings());
        assert!(matches!(result, Some(runie_core::Event::Undo)), "Ctrl+Z should map to Undo");
    }

    #[test]
    fn ctrl_y_converts_to_redo() {
        let key = KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL);
        let event = crossterm::event::Event::Key(key);
        let result = super::convert_event(&event, &default_bindings());
        assert!(matches!(result, Some(runie_core::Event::Redo)), "Ctrl+Y should map to Redo");
    }

    #[test]
    fn alt_b_converts_to_word_left() {
        let key = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::ALT);
        let event = crossterm::event::Event::Key(key);
        let result = super::convert_event(&event, &default_bindings());
        assert!(matches!(result, Some(runie_core::Event::CursorWordLeft)), "Alt+B should map to CursorWordLeft");
    }

    #[test]
    fn alt_f_converts_to_word_right() {
        let key = KeyEvent::new(KeyCode::Char('f'), KeyModifiers::ALT);
        let event = crossterm::event::Event::Key(key);
        let result = super::convert_event(&event, &default_bindings());
        assert!(matches!(result, Some(runie_core::Event::CursorWordRight)), "Alt+F should map to CursorWordRight");
    }

    #[test]
    fn bracketed_paste_converts_to_paste_event() {
        let event = crossterm::event::Event::Paste("hello world".to_string());
        let result = super::convert_event(&event, &default_bindings());
        assert!(matches!(result, Some(runie_core::Event::Paste(s)) if s == "hello world"), "Paste event should map to CoreEvent::Paste");
    }

    #[test]
    fn custom_keybinding_overrides_default() {
        let mut bindings = default_bindings();
        bindings.insert("ctrl+c".to_string(), "Abort".to_string());
        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        let event = crossterm::event::Event::Key(key);
        let result = super::convert_event(&event, &bindings);
        assert!(matches!(result, Some(runie_core::Event::Abort)), "Custom keybinding should override default");
    }

    #[test]
    fn unknown_keybinding_falls_back_to_default() {
        let bindings = default_bindings();
        let key = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL);
        let event = crossterm::event::Event::Key(key);
        let result = super::convert_event(&event, &bindings);
        assert_eq!(result, None, "Unmapped key should fall through to None");
    }

    #[test]
    fn key_event_to_combo_ctrl_c() {
        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(super::key_event_to_combo(&key), "ctrl+c");
    }

    #[test]
    fn key_event_to_combo_alt_enter() {
        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::ALT);
        assert_eq!(super::key_event_to_combo(&key), "alt+enter");
    }

    #[test]
    fn alt_up_emits_dequeue() {
        let key = KeyEvent::new(KeyCode::Up, KeyModifiers::ALT);
        let event = crossterm::event::Event::Key(key);
        let result = super::convert_event(&event, &default_bindings());
        assert!(matches!(result, Some(runie_core::Event::Dequeue)), "Alt+Up should map to Dequeue, got {:?}", result);
    }

    #[test]
    fn ctrl_g_emits_open_external_editor() {
        let key = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL);
        let event = crossterm::event::Event::Key(key);
        let result = super::convert_event(&event, &default_bindings());
        assert!(matches!(result, Some(runie_core::Event::OpenExternalEditor)), "Ctrl+G should map to OpenExternalEditor, got {:?}", result);
    }

    #[test]
    fn key_event_to_combo_shift_enter() {
        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT);
        assert_eq!(super::key_event_to_combo(&key), "shift+enter");
    }

    #[test]
    fn key_event_to_combo_plain_escape() {
        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
        assert_eq!(super::key_event_to_combo(&key), "escape");
    }
}
