use std::collections::HashMap;

/// Default keybindings — data formerly in `resources/keybindings/default.yaml`,
/// now expressed directly in Rust. Platform-conditional bindings are gated
/// with `#[cfg(...)]`.
pub fn default_keybindings() -> HashMap<String, String> {
    let mut map = HashMap::new();
    // ── All platforms ─────────────────────────────────────────────────────────
    map.insert("ctrl+e".into(), "CursorEnd".into());
    map.insert("ctrl+o".into(), "ToggleExpand".into());
    map.insert("ctrl+j".into(), "Newline".into());
    map.insert("ctrl+a".into(), "CursorStart".into());
    map.insert("ctrl+b".into(), "ToggleTasksPane".into());
    map.insert("ctrl+f".into(), "CursorRight".into());
    map.insert("ctrl+w".into(), "DeleteWord".into());
    map.insert("ctrl+k".into(), "DeleteToEnd".into());
    map.insert("ctrl+u".into(), "DeleteToStart".into());
    map.insert("ctrl+d".into(), "KillChar".into());
    map.insert("ctrl+z".into(), "Suspend".into());
    map.insert("ctrl+y".into(), "Redo".into());
    map.insert("ctrl+c".into(), "Quit".into());
    map.insert("ctrl+q".into(), "ForceQuit".into());
    map.insert("ctrl+s".into(), "Abort".into());
    map.insert("ctrl+g".into(), "OpenExternalEditor".into());
    map.insert("ctrl+shift+o".into(), "CopyLastResponse".into());
    map.insert("ctrl+p".into(), "ToggleCommandPalette".into());
    map.insert("ctrl+shift+p".into(), "ToggleCommandPalette".into());
    map.insert("ctrl+n".into(), "NewSession".into());
    map.insert("ctrl+r".into(), "ResumeSession".into());
    map.insert("ctrl+m".into(), "CycleModelNext".into());
    map.insert("ctrl+shift+m".into(), "CycleModelPrev".into());
    map.insert("alt+enter".into(), "FollowUp".into());
    map.insert("alt+up".into(), "Dequeue".into());
    map.insert("alt+b".into(), "CursorWordLeft".into());
    map.insert("alt+f".into(), "CursorWordRight".into());
    map.insert("escape".into(), "DialogBack".into());
    map.insert("tab".into(), "Input:\t".into());
    map.insert("backspace".into(), "Backspace".into());
    map.insert("enter".into(), "Submit".into());
    map.insert("up".into(), "HistoryPrev".into());
    map.insert("down".into(), "HistoryNext".into());
    map.insert("left".into(), "CursorLeft".into());
    map.insert("right".into(), "CursorRight".into());
    map.insert("home".into(), "CursorStart".into());
    map.insert("end".into(), "CursorEnd".into());
    map.insert("delete".into(), "KillChar".into());
    map.insert("shift+enter".into(), "Newline".into());
    map.insert("shift+tab".into(), "CycleThinkingLevel".into());
    map.insert("pageup".into(), "PageUp".into());
    map.insert("pagedown".into(), "PageDown".into());
    // ── Platform-conditional ─────────────────────────────────────────────────
    #[cfg(not(target_os = "windows"))]
    map.insert("ctrl+v".into(), "PasteImage".into());
    #[cfg(target_os = "windows")]
    map.insert("alt+v".into(), "PasteImage".into());
    map
}
