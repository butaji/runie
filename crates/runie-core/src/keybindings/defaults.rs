use std::collections::HashMap;

/// Valid final key names for key combos.
pub const VALID_KEYS: &[&str] = &[
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
pub const DEFAULT_BINDINGS: &[(&str, &str)] = &[
    ("ctrl+e", "CursorEnd"),
    ("ctrl+o", "ToggleExpand"),
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
    ("ctrl+shift+o", "CopyLastResponse"),
    ("ctrl+p", "ToggleCommandPalette"),
    ("ctrl+shift+p", "ToggleCommandPalette"),
    ("ctrl+n", "NewSession"),
    ("ctrl+r", "ResumeSession"),
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
