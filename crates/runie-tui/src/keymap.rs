//! Crossterm key event → CoreEvent conversion with configurable keybindings.
//!
//! Uses a single HashMap lookup for both default and user keybindings.
//! The default map is built once at startup from the YAML defaults.

use crokey::KeyCombination;
use crokey::KeyCombinationFormat;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use runie_core::{keybindings, Event as CoreEvent};
use std::collections::HashMap;
use std::sync::LazyLock;

/// Lowercase `+`-separated combo formatter backed by `crokey::KeyCombinationFormat`.
/// Output matches the legacy format: "ctrl+c", "alt+enter", "shift+tab".
static COMBO_FORMAT: LazyLock<KeyCombinationFormat> = LazyLock::new(|| {
    KeyCombinationFormat::default()
        .with_lowercase_modifiers()
        .with_control("ctrl-")
        .with_alt("alt-")
        .with_shift("shift-")
        .with_command("cmd-")
});

/// Default keybindings map built once at startup.
/// Maps KeyCombination string → CoreEvent for all default bindings.
static DEFAULT_MAP: LazyLock<HashMap<String, CoreEvent>> = LazyLock::new(|| {
    let defaults = keybindings::default_keybindings();
    let mut map = HashMap::new();
    for (combo, event_name) in defaults {
        if let Some(event) = keybindings::event_from_name(&event_name) {
            map.insert(combo, event);
        } else {
            tracing::warn!("Unknown event name in default keybindings: {}", event_name);
        }
    }
    map
});

pub fn convert_event(event: &Event, user_bindings: &HashMap<String, String>) -> Option<CoreEvent> {
    log_key_event(event);
    match event {
        Event::Paste(data) => Some(CoreEvent::Paste(data.clone())),
        // Mouse capture is never enabled, so these events cannot arrive;
        // drop them defensively (native terminal selection owns the mouse).
        Event::Mouse(_) => None,
        Event::FocusGained => Some(CoreEvent::FocusGained),
        Event::FocusLost => Some(CoreEvent::FocusLost),
        Event::Resize(width, height) => Some(CoreEvent::TerminalSize {
            width: *width,
            height: *height,
        }),
        Event::Key(key) if is_press_or_repeat(key) => convert_key_event(key, user_bindings),
        _ => None,
    }
}

fn is_press_or_repeat(key: &KeyEvent) -> bool {
    key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat
}

fn convert_key_event(key: &KeyEvent, user_bindings: &HashMap<String, String>) -> Option<CoreEvent> {
    // Special handling for Enter variants
    if key.modifiers.is_empty() && key.code == KeyCode::Char('\n') {
        return Some(CoreEvent::Newline);
    }
    if key.modifiers.contains(KeyModifiers::SHIFT) && is_enter_like(key.code) {
        return Some(CoreEvent::Newline);
    }
    if key.code == KeyCode::F(3) {
        return Some(CoreEvent::Newline);
    }

    map_key_event(key, user_bindings)
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
            let key = *key;
            tokio::task::spawn_blocking(move || {
                use std::io::Write;
                let _ = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("/tmp/runie_keys.log")
                    .and_then(|mut f| writeln!(f, "{:?}", key));
            });
        }
    }
}

/// Handle escape sequences that crossterm doesn't parse as KeyEvent.
/// Many terminals send different sequences for modified keys.
pub fn key_event_to_combo(key: &KeyEvent) -> String {
    match key.code {
        // crokey formats KeyCode::Esc as "Esc" via Debug; map to "escape".
        KeyCode::Esc if key.modifiers.is_empty() => return "escape".to_owned(),
        // BackTab is always SHIFT+Tab in the binding table; use the legacy alias.
        KeyCode::BackTab => return "shift+tab".to_owned(),
        _ => {}
    }
    let combo = KeyCombination::from(*key);
    COMBO_FORMAT
        .to_string(combo)
        .to_lowercase()
        .replace('-', "+")
}

/// Map a key event to a CoreEvent using the binding maps.
/// Priority: 1. User bindings override defaults, 2. Default map, 3. Plain key fallback
fn map_key_event(key: &KeyEvent, user_bindings: &HashMap<String, String>) -> Option<CoreEvent> {
    let combo = key_event_to_combo(key);
    if combo.is_empty() {
        return map_plain_key(&key.code);
    }

    // Special case: Ctrl+Shift+E is not a binding (it's paste special for image paste)
    // Check this before any binding lookup since it's not in the default map
    if key.modifiers.contains(KeyModifiers::CONTROL)
        && key.modifiers.contains(KeyModifiers::SHIFT)
        && matches!(key.code, KeyCode::Char('e') | KeyCode::Char('E'))
    {
        return None;
    }

    // 1. Check user bindings first (they override defaults)
    if let Some(event_name) = user_bindings.get(&combo) {
        return keybindings::event_from_name(event_name);
    }

    // 2. Check default map
    if let Some(event) = DEFAULT_MAP.get(&combo) {
        return Some(event.clone());
    }

    // 3. Fall back to plain key handling for unhandled keys
    map_plain_key(&key.code)
}

/// Fallback handler for plain keys not in any binding map.
fn map_plain_key(code: &KeyCode) -> Option<CoreEvent> {
    match code {
        // Esc acts as a **Back button** in any open dialog (command bar,
        // settings, login flow, model selector, etc.). The dialog's
        // panel-stack handler interprets `DialogBack` as stack nav:
        // pop one panel when deeper, close the dialog when at the root
        // (the "main menu" of that bar). To force-close from any depth
        // use `Abort` (Ctrl+\) instead.
        KeyCode::Esc => Some(CoreEvent::DialogBack),
        KeyCode::Char('\t') | KeyCode::Tab | KeyCode::BackTab => Some(CoreEvent::Input('\t')),
        KeyCode::Char(c) => Some(CoreEvent::Input(*c)),
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
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn test_ctrl_c_maps_to_quit() {
        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        let user_bindings = HashMap::new();
        let result = convert_key_event(&key, &user_bindings);
        assert_eq!(result, Some(CoreEvent::Quit));
    }

    #[test]
    fn test_ctrl_e_maps_to_cursor_end() {
        let key = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL);
        let user_bindings = HashMap::new();
        let result = convert_key_event(&key, &user_bindings);
        assert_eq!(result, Some(CoreEvent::CursorEnd));
    }

    #[test]
    fn test_ctrl_shift_e_is_ignored() {
        let key = KeyEvent::new(
            KeyCode::Char('E'),
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        );
        let user_bindings = HashMap::new();
        let result = convert_key_event(&key, &user_bindings);
        assert_eq!(result, None);
    }

    #[test]
    fn test_escape_maps_to_dialog_back() {
        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
        let user_bindings = HashMap::new();
        let result = convert_key_event(&key, &user_bindings);
        assert_eq!(result, Some(CoreEvent::DialogBack));
    }

    #[test]
    fn test_char_input_maps_to_input() {
        let key = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::empty());
        let user_bindings = HashMap::new();
        let result = convert_key_event(&key, &user_bindings);
        assert_eq!(result, Some(CoreEvent::Input('x')));
    }

    #[test]
    fn test_user_binding_overrides_default() {
        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        let mut user_bindings = HashMap::new();
        user_bindings.insert("ctrl+c".to_owned(), "Abort".to_owned());
        let result = convert_key_event(&key, &user_bindings);
        assert_eq!(result, Some(CoreEvent::Abort));
    }

    #[test]
    fn test_alt_enter_maps_to_follow_up() {
        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::ALT);
        let user_bindings = HashMap::new();
        let result = convert_key_event(&key, &user_bindings);
        assert_eq!(result, Some(CoreEvent::FollowUp));
    }

    #[test]
    fn test_up_arrow_maps_to_history_prev() {
        let key = KeyEvent::new(KeyCode::Up, KeyModifiers::empty());
        let user_bindings = HashMap::new();
        let result = convert_key_event(&key, &user_bindings);
        assert_eq!(result, Some(CoreEvent::HistoryPrev));
    }

    #[test]
    fn test_backspace_maps_to_backspace() {
        let key = KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty());
        let user_bindings = HashMap::new();
        let result = convert_key_event(&key, &user_bindings);
        assert_eq!(result, Some(CoreEvent::Backspace));
    }

    #[test]
    fn test_delete_maps_to_kill_char() {
        let key = KeyEvent::new(KeyCode::Delete, KeyModifiers::empty());
        let user_bindings = HashMap::new();
        let result = convert_key_event(&key, &user_bindings);
        assert_eq!(result, Some(CoreEvent::KillChar));
    }
}

// The `keymap/tests/` directory was previously orphaned: this file declares an
// inline `mod tests`, which shadows the directory and left `merge.rs` (mouse
// drop / terminal-caps merge coverage) uncompiled. Mount it explicitly so
// those tests actually run.
#[cfg(test)]
#[path = "keymap/tests/merge.rs"]
mod keymap_merge_tests;
