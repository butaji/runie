//! Configurable keybindings module.
//!
//! Loads keybindings from `~/.runie/keybindings.json` and provides
//! fallback defaults if file doesn't exist or is invalid.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::event::Event;

/// Valid final key names for key combos.
const VALID_KEYS: &[&str] = &[
    "a",
    "b",
    "c",
    "d",
    "e",
    "f",
    "g",
    "h",
    "i",
    "j",
    "k",
    "l",
    "m",
    "n",
    "o",
    "p",
    "q",
    "r",
    "s",
    "t",
    "u",
    "v",
    "w",
    "x",
    "y",
    "z",
    "0",
    "1",
    "2",
    "3",
    "4",
    "5",
    "6",
    "7",
    "8",
    "9",
    "backspace",
    "enter",
    "escape",
    "tab",
    "up",
    "down",
    "left",
    "right",
    "home",
    "end",
    "delete",
    "f1",
    "f2",
    "f3",
    "f4",
    "f5",
    "f6",
    "f7",
    "f8",
    "f9",
    "f10",
    "f11",
    "f12",
    "pageup",
    "pagedown",
    "space",
];

/// Default bindings as (combo, event_name) tuples.
const DEFAULT_BINDINGS: &[(&str, &str)] = &[
    ("ctrl+e", "CursorEnd"),
    ("ctrl+shift+e", "ToggleExpand"),
    ("ctrl+j", "Newline"),
    ("ctrl+a", "CursorStart"),
    ("ctrl+b", "CursorLeft"),
    ("ctrl+f", "CursorRight"),
    ("ctrl+w", "DeleteWord"),
    ("ctrl+k", "DeleteToEnd"),
    ("ctrl+u", "DeleteToStart"),
    ("ctrl+d", "KillChar"),
    ("ctrl+z", "Suspend"),
    ("ctrl+y", "Redo"),
    ("ctrl+c", "Quit"),
    ("ctrl+q", "Quit"),
    ("ctrl+s", "Abort"),
    ("ctrl+g", "OpenExternalEditor"),
    ("ctrl+o", "CopyLastResponse"),
    ("ctrl+p", "ToggleCommandPalette"),
    ("ctrl+shift+p", "ToggleCommandPalette"),
    ("ctrl+m", "CycleModelNext"),
    ("ctrl+shift+m", "CycleModelPrev"),
    ("alt+enter", "FollowUp"),
    ("alt+up", "Dequeue"),
    ("alt+b", "CursorWordLeft"),
    ("alt+f", "CursorWordRight"),
    ("escape", "DialogBack"),
    ("tab", "Input:\t"),
    ("backspace", "Backspace"),
    ("enter", "Submit"),
    ("up", "HistoryPrev"),
    ("down", "HistoryNext"),
    ("left", "CursorLeft"),
    ("right", "CursorRight"),
    ("home", "CursorStart"),
    ("end", "CursorEnd"),
    ("delete", "KillChar"),
    ("shift+enter", "Newline"),
    ("shift+tab", "CycleThinkingLevel"),
    ("pageup", "PageUp"),
    ("pagedown", "PageDown"),
];

/// Default keybindings map (key combo string → event name)
pub fn default_keybindings() -> HashMap<String, String> {
    let mut map = HashMap::new();
    for (combo, name) in DEFAULT_BINDINGS {
        map.insert(combo.to_string(), name.to_string());
    }
    #[cfg(not(target_os = "windows"))]
    {
        map.insert("ctrl+v".to_string(), "PasteImage".to_string());
    }
    #[cfg(target_os = "windows")]
    {
        map.insert("alt+v".to_string(), "PasteImage".to_string());
    }
    map
}

/// Parse a key combination string to components
/// Examples: "ctrl+c", "alt+enter", "shift+up"
#[allow(dead_code)]
fn parse_key_combo(combo: &str) -> (Vec<String>, String) {
    let lower = combo.to_lowercase();
    let parts: Vec<&str> = lower.split('+').collect();
    if parts.is_empty() {
        return (vec![], String::new());
    }
    let key = parts[parts.len() - 1].to_string();
    let modifiers: Vec<String> = parts[..parts.len() - 1]
        .iter()
        .map(|s| s.to_string())
        .collect();
    (modifiers, key)
}

/// Load keybindings from file, falling back to defaults
pub fn load_keybindings(path: &Option<PathBuf>) -> HashMap<String, String> {
    let path = match path {
        Some(p) => p.clone(),
        None => default_keybindings_path()
            .unwrap_or_else(|| PathBuf::from("/tmp/runie_keybindings.json")),
    };

    if !path.exists() {
        return default_keybindings();
    }

    match fs::read_to_string(&path) {
        Ok(content) => parse_keybindings_json(&content).unwrap_or_else(|e| {
            eprintln!("Failed to parse keybindings: {}, using defaults", e);
            default_keybindings()
        }),
        Err(e) => {
            eprintln!("Failed to read keybindings file: {}, using defaults", e);
            default_keybindings()
        }
    }
}

/// Parse keybindings from JSON string
pub fn parse_keybindings_json(content: &str) -> Result<HashMap<String, String>> {
    let value: serde_json::Value =
        serde_json::from_str(content).context("parse keybindings JSON")?;

    let obj = value.as_object().context("keybindings must be an object")?;

    let mut bindings = default_keybindings(); // Start with defaults

    for (key, val) in obj {
        let event_name = val
            .as_str()
            .context(format!("binding value for '{}' must be a string", key))?
            .to_string();
        bindings.insert(key.to_lowercase(), event_name);
    }

    Ok(bindings)
}

/// Convert an event name string to an Event variant.
/// Supports simple names (e.g. "Quit", "Submit") and Input prefix (e.g. "Input:\t").
pub fn event_from_name(name: &str) -> Option<Event> {
    Event::from_name(name)
}

/// Get default keybindings file path
pub fn default_keybindings_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("runie").join("keybindings.json"))
}

/// Validate that a key combo string is well-formed
pub fn validate_key_combo(combo: &str) -> bool {
    let parts: Vec<&str> = combo.split('+').collect();
    if parts.is_empty() || parts.len() > 3 {
        return false;
    }
    let key = parts[parts.len() - 1];
    VALID_KEYS.contains(&key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_keybindings_has_common_keys() {
        let bindings = default_keybindings();
        assert_eq!(bindings.get("ctrl+c"), Some(&"Quit".to_string()));
        assert_eq!(bindings.get("ctrl+z"), Some(&"Suspend".to_string()));
        assert_eq!(bindings.get("enter"), Some(&"Submit".to_string()));
        assert_eq!(bindings.get("up"), Some(&"HistoryPrev".to_string()));
    }

    #[test]
    fn ctrl_e_defaults_to_cursor_end() {
        let bindings = default_keybindings();
        assert_eq!(
            bindings.get("ctrl+e"),
            Some(&"CursorEnd".to_string()),
            "ctrl+e should move cursor to end of input"
        );
    }

    #[test]
    fn ctrl_shift_e_defaults_to_toggle_expand() {
        let bindings = default_keybindings();
        assert_eq!(
            bindings.get("ctrl+shift+e"),
            Some(&"ToggleExpand".to_string()),
            "ctrl+shift+e should expand/collapse thoughts"
        );
    }

    #[test]
    fn ctrl_q_defaults_to_quit() {
        let bindings = default_keybindings();
        assert_eq!(
            bindings.get("ctrl+q"),
            Some(&"Quit".to_string()),
            "ctrl+q should quit the app"
        );
    }

    #[test]
    fn parse_key_combo_ctrl_c() {
        let (mods, key) = parse_key_combo("ctrl+c");
        assert_eq!(mods, vec!["ctrl"]);
        assert_eq!(key, "c");
    }

    #[test]
    fn parse_key_combo_alt_enter() {
        let (mods, key) = parse_key_combo("alt+enter");
        assert_eq!(mods, vec!["alt"]);
        assert_eq!(key, "enter");
    }

    #[test]
    fn parse_key_combo_plain() {
        let (mods, key) = parse_key_combo("enter");
        assert!(mods.is_empty());
        assert_eq!(key, "enter");
    }

    #[test]
    fn parse_key_combo_shift_enter() {
        let (mods, key) = parse_key_combo("shift+enter");
        assert_eq!(mods, vec!["shift"]);
        assert_eq!(key, "enter");
    }

    #[test]
    fn load_keybindings_falls_back_to_defaults() {
        // Non-existent path should return defaults
        let path = PathBuf::from("/non/existent/path.json");
        let bindings = load_keybindings(&Some(path));
        assert_eq!(bindings.get("ctrl+c"), Some(&"Quit".to_string()));
    }

    #[test]
    fn parse_keybindings_json_with_overrides() {
        let json = r#"{
            "ctrl+x": "Quit",
            "ctrl+q": "Undo"
        }"#;
        let bindings = parse_keybindings_json(json).unwrap();

        // Overridden
        assert_eq!(bindings.get("ctrl+x"), Some(&"Quit".to_string()));
        assert_eq!(bindings.get("ctrl+q"), Some(&"Undo".to_string()));

        // Still has defaults
        assert_eq!(bindings.get("ctrl+z"), Some(&"Suspend".to_string()));
    }

    #[test]
    fn parse_keybindings_json_invalid_json() {
        let json = "not valid json";
        let result = parse_keybindings_json(json);
        assert!(result.is_err());
    }

    #[test]
    fn validate_key_combo_valid() {
        assert!(validate_key_combo("ctrl+c"));
        assert!(validate_key_combo("alt+enter"));
        assert!(validate_key_combo("shift+up"));
        assert!(validate_key_combo("escape"));
    }

    #[test]
    fn validate_key_combo_invalid() {
        assert!(!validate_key_combo(""));
        assert!(!validate_key_combo("too+many+modifiers+here"));
    }

    #[test]
    fn keybindings_load_default_when_file_missing() {
        // Test that load_keybindings returns defaults when file doesn't exist
        let bindings = load_keybindings(&Some(PathBuf::from("/tmp/nonexistent_keybindings.json")));
        // Should contain default bindings
        assert!(bindings.contains_key("ctrl+c"));
    }

    #[test]
    fn event_from_name_quit() {
        assert!(matches!(event_from_name("Quit"), Some(Event::Quit)));
    }

    #[test]
    fn event_from_name_submit() {
        assert!(matches!(event_from_name("Submit"), Some(Event::Submit)));
    }

    #[test]
    fn event_from_name_input_tab() {
        assert!(matches!(
            event_from_name("Input:\t"),
            Some(Event::Input('\t'))
        ));
    }

    #[test]
    fn event_from_name_input_char() {
        assert!(matches!(
            event_from_name("Input:a"),
            Some(Event::Input('a'))
        ));
    }

    #[test]
    fn event_from_name_unknown_returns_none() {
        assert_eq!(event_from_name("UnknownEvent"), None);
    }

    #[test]
    fn event_from_name_all_named_variants() {
        for (name, ctor) in crate::event::EVENT_NAMES {
            let expected = ctor();
            let actual = event_from_name(name).expect(name);
            assert!(
                std::mem::discriminant(&actual) == std::mem::discriminant(&expected),
                "event_from_name({:?}) returned wrong variant",
                name
            );
        }
    }

    #[test]
    fn default_keybindings_resolve() {
        let bindings = default_keybindings();
        for name in bindings.values() {
            assert!(
                event_from_name(name).is_some(),
                "default binding {} does not resolve",
                name
            );
        }
    }

    #[test]
    fn validate_key_combo_accepts_default_keys() {
        let bindings = default_keybindings();
        for combo in bindings.keys() {
            assert!(
                validate_key_combo(combo),
                "default combo {} rejected",
                combo
            );
        }
    }

    #[test]
    fn invalid_key_combo_rejected() {
        assert!(!validate_key_combo("ctrl+💩"));
    }
}
