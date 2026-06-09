//! Configurable keybindings module.
//!
//! Loads keybindings from `~/.runie/keybindings.json` and provides
//! fallback defaults if file doesn't exist or is invalid.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::event::Event;
use crate::labels;

/// Default keybindings map (key combo string → event name)
pub fn default_keybindings() -> HashMap<String, String> {
    let mut map = HashMap::new();

    // Control key combinations
    map.insert("ctrl+e".to_string(), "ToggleExpand".to_string());
    map.insert("ctrl+j".to_string(), "Newline".to_string());
    map.insert("ctrl+a".to_string(), "CursorStart".to_string());
    map.insert("ctrl+b".to_string(), "CursorLeft".to_string());
    map.insert("ctrl+f".to_string(), "CursorRight".to_string());
    map.insert("ctrl+w".to_string(), "DeleteWord".to_string());
    map.insert("ctrl+k".to_string(), "DeleteToEnd".to_string());
    map.insert("ctrl+u".to_string(), "DeleteToStart".to_string());
    map.insert("ctrl+d".to_string(), "KillChar".to_string());
    map.insert("ctrl+z".to_string(), "Undo".to_string());
    map.insert("ctrl+y".to_string(), "Redo".to_string());
    map.insert("ctrl+c".to_string(), "Quit".to_string());
    map.insert("ctrl+s".to_string(), "Abort".to_string());

    // Alt key combinations
    map.insert("alt+enter".to_string(), "FollowUp".to_string());
    map.insert("alt+b".to_string(), "CursorWordLeft".to_string());
    map.insert("alt+f".to_string(), "CursorWordRight".to_string());

    // Plain keys
    map.insert("escape".to_string(), "Abort".to_string());
    map.insert("tab".to_string(), "Input:\\t".to_string());
    map.insert("backspace".to_string(), "Backspace".to_string());
    map.insert("enter".to_string(), "Submit".to_string());
    map.insert("up".to_string(), "HistoryPrev".to_string());
    map.insert("down".to_string(), "HistoryNext".to_string());
    map.insert("left".to_string(), "CursorLeft".to_string());
    map.insert("right".to_string(), "CursorRight".to_string());
    map.insert("home".to_string(), "CursorStart".to_string());
    map.insert("end".to_string(), "CursorEnd".to_string());
    map.insert("delete".to_string(), "KillChar".to_string());

    // Shift combinations (handled specially)
    map.insert("shift+enter".to_string(), "Newline".to_string());

    map
}

/// Parse a key combination string to components
/// Examples: "ctrl+c", "alt+enter", "shift+up"
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
        None => default_keybindings_path().unwrap_or_else(|| PathBuf::from("/tmp/runie_keybindings.json")),
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
    let value: serde_json::Value = serde_json::from_str(content)
        .context("parse keybindings JSON")?;

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
    // Last part must be a valid key
    let key = parts[parts.len() - 1];
    matches!(key,
        "a" | "b" | "c" | "d" | "e" | "f" | "g" | "h" | "i" | "j" | "k" | "l" | "m"
        | "n" | "o" | "p" | "q" | "r" | "s" | "t" | "u" | "v" | "w" | "x" | "y" | "z"
        | "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"
        | "backspace" | "enter" | "escape" | "tab"
        | "up" | "down" | "left" | "right" | "home" | "end" | "delete"
        | "f1" | "f2" | "f3" | "f4" | "f5" | "f6" | "f7" | "f8" | "f9" | "f10" | "f11" | "f12"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_keybindings_has_common_keys() {
        let bindings = default_keybindings();
        assert_eq!(bindings.get("ctrl+c"), Some(&"Quit".to_string()));
        assert_eq!(bindings.get("ctrl+z"), Some(&"Undo".to_string()));
        assert_eq!(bindings.get("enter"), Some(&"Submit".to_string()));
        assert_eq!(bindings.get("up"), Some(&"HistoryPrev".to_string()));
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
        assert_eq!(bindings.get("ctrl+z"), Some(&"Undo".to_string()));
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
}
