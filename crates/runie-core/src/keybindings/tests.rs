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
fn ctrl_o_defaults_to_toggle_expand() {
    let bindings = default_keybindings();
    assert_eq!(
        bindings.get("ctrl+o"),
        Some(&"ToggleExpand".to_string()),
        "ctrl+o should collapse/expand feed posts"
    );
}

#[test]
fn ctrl_shift_e_has_no_default_binding() {
    let bindings = default_keybindings();
    assert!(
        !bindings.contains_key("ctrl+shift+e"),
        "ctrl+shift+e should not have a default binding; use ctrl+o instead"
    );
}

#[test]
fn ctrl_shift_o_defaults_to_copy_last_response() {
    let bindings = default_keybindings();
    assert_eq!(
        bindings.get("ctrl+shift+o"),
        Some(&"CopyLastResponse".to_string()),
        "ctrl+shift+o should copy the last assistant response"
    );
}

#[test]
fn ctrl_q_defaults_to_force_quit() {
    let bindings = default_keybindings();
    assert_eq!(
        bindings.get("ctrl+q"),
        Some(&"ForceQuit".to_string()),
        "ctrl+q should force-quit the app"
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
    // No config → falls back to defaults
    let bindings = load_keybindings(None);
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
    // No config → load_keybindings uses defaults
    let bindings = load_keybindings(None);
    assert!(bindings.contains_key("ctrl+c"));
}

#[test]
fn event_from_name_quit() {
    assert!(matches!(event_from_name("Quit"), Some(crate::Event::Quit)));
}

#[test]
fn event_from_name_submit() {
    assert!(matches!(
        event_from_name("Submit"),
        Some(crate::Event::Submit)
    ));
}

#[test]
fn event_from_name_input_tab() {
    assert!(matches!(
        event_from_name("Input:\t"),
        Some(crate::Event::Input('\t'))
    ));
}

#[test]
fn event_from_name_input_char() {
    assert!(matches!(
        event_from_name("Input:a"),
        Some(crate::Event::Input('a'))
    ));
}

#[test]
fn event_from_name_unknown_returns_none() {
    assert_eq!(event_from_name("UnknownEvent"), None);
}

#[test]
fn event_name_roundtrip() {
    for (name, ctor) in crate::event::EVENT_NAMES {
        let evt = ctor();
        if let Some(got) = evt.name() {
            assert_eq!(got, *name, "{}: Event::name() returned wrong name", name);
        }
    }
}

#[test]
fn mouse_events_have_no_name() {
    assert_eq!(
        crate::Event::MouseClick {
            row: 0,
            col: 0,
            button: "left".into()
        }
        .name(),
        None
    );
    assert_eq!(
        crate::Event::MouseRelease {
            row: 0,
            col: 0,
            button: "left".into()
        }
        .name(),
        None
    );
    assert_eq!(
        crate::Event::MouseDrag {
            row: 0,
            col: 0,
            button: "left".into()
        }
        .name(),
        None
    );
    assert_eq!(crate::Event::MouseMove { row: 0, col: 0 }.name(), None);
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

// -------------------------------------------------------------------------
// Layer 1 tests: config keybindings integration
// -------------------------------------------------------------------------

#[test]
fn config_keybindings_override_defaults() {
    let mut config = crate::config::Config::default();
    config
        .keybindings
        .insert("ctrl+c".to_string(), "Abort".to_string());

    let bindings = merged_keybindings(&config);

    // Overridden binding resolves to Abort
    assert_eq!(
        bindings.get("ctrl+c"),
        Some(&"Abort".to_string()),
        "ctrl+c should be overridden to Abort"
    );
    // Other defaults remain unchanged
    assert_eq!(
        bindings.get("ctrl+z"),
        Some(&"Suspend".to_string()),
        "ctrl+z default should remain"
    );
}

#[test]
fn config_keybindings_merge_with_defaults() {
    let config = crate::config::Config::default();
    let bindings = merged_keybindings(&config);

    // All defaults are present
    assert_eq!(bindings.get("ctrl+c"), Some(&"Quit".to_string()));
    assert_eq!(bindings.get("enter"), Some(&"Submit".to_string()));
    assert_eq!(bindings.get("ctrl+o"), Some(&"ToggleExpand".to_string()));
}

#[test]
fn config_keybindings_empty_map_uses_all_defaults() {
    let mut config = crate::config::Config::default();
    config.keybindings = std::collections::HashMap::new();
    let bindings = merged_keybindings(&config);

    assert_eq!(
        bindings.len(),
        default_keybindings().len(),
        "empty keybindings map should leave all defaults"
    );
}

#[test]
fn keybindings_json_migration() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    let json_path = dir.path().join("keybindings.json");

    // Write legacy JSON file
    let json = r#"{"ctrl+x": "Quit", "ctrl+q": "Undo"}"#;
    std::fs::write(&json_path, json).unwrap();

    // Write initial config (v2 so migration triggers)
    std::fs::write(&config_path, "version = 2\nprovider = \"openai\"\n").unwrap();

    // Parse and migrate with the temp config path
    let mut value: toml::Value =
        toml::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
    crate::config_migrate::migrate_with_path(&mut value, Some(config_path.clone())).unwrap();

    // After migration, keybindings should be in config
    assert!(
        value.get("keybindings").is_some(),
        "migrated config should contain [keybindings] table"
    );
    // And JSON file should be renamed to .bak
    assert!(
        json_path.with_extension("json.bak").exists(),
        "keybindings.json should be renamed to .bak"
    );
}

#[test]
fn invalid_key_combo_rejected() {
    assert!(!validate_key_combo("ctrl+💩"));
}
