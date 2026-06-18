//! Settings dialog tests (Layer 2 + Layer 3)

use runie_core::commands::DialogState;
use runie_core::event::{ControlEvent, DialogEvent, InputEvent, ModelConfigEvent};
use runie_core::model::{AppState, DeliveryMode};
use runie_core::settings::{SettingValue, SettingsCategory};
use runie_core::update::settings_dialog::build_setting_items;

fn settings_selected(state: &AppState) -> Option<usize> {
    match &state.open_dialog {
        Some(DialogState::Settings(stack)) => stack.current().map(|p| p.selected),
        _ => None,
    }
}

fn settings_count(state: &AppState) -> usize {
    build_setting_items(state).len()
}

/// Open palette and select a command by name
fn palette_select(state: &mut AppState, cmd: &str) {
    state.update(InputEvent::Input('/'));
    for c in cmd.chars() {
        state.update(DialogEvent::PaletteFilter(c));
    }
    state.update(DialogEvent::PaletteSelect);
}

#[test]
fn settings_opens_dialog() {
    let mut state = AppState::default();
    palette_select(&mut state, "settings");
    assert!(
        matches!(state.open_dialog, Some(DialogState::Settings(_))),
        "Expected Settings dialog, got {:?}",
        state.open_dialog
    );
}

#[test]
fn settings_navigates_up() {
    let mut state = AppState::default();
    state.update(DialogEvent::ToggleSettingsDialog);
    state.update(ModelConfigEvent::SettingsUp);
    let selected = settings_selected(&state).expect("Dialog should still be open");
    assert_eq!(
        selected,
        settings_count(&state) - 1,
        "Up at first wraps to last"
    );
}

#[test]
fn settings_navigates_down_wraps() {
    let mut state = AppState::default();
    state.update(DialogEvent::ToggleSettingsDialog);
    let count = settings_count(&state);
    for _ in 0..count {
        state.update(ModelConfigEvent::SettingsDown);
    }
    let selected = settings_selected(&state).expect("Dialog should still be open");
    assert_eq!(selected, 0, "Down wraps to first");
}

#[test]
fn settings_select_toggles_read_only() {
    let mut state = AppState::default();
    state.config.read_only = false;
    state.update(DialogEvent::ToggleSettingsDialog);
    // Scan for the read-only toggle
    let count = settings_count(&state);
    for _ in 0..count {
        let is_readonly = if let Some(DialogState::Settings(stack)) = &state.open_dialog {
            stack
                .current()
                .and_then(|p| p.selected_item())
                .and_then(|i| i.label())
                == Some("Read-Only")
        } else {
            false
        };
        if is_readonly {
            state.update(ModelConfigEvent::SettingsSelect);
            break;
        }
        state.update(ModelConfigEvent::SettingsDown);
    }
    assert!(state.config.read_only);
}

#[test]
fn settings_space_toggles_read_only_and_keeps_dialog_open() {
    let mut state = AppState::default();
    state.config.read_only = false;
    state.update(DialogEvent::ToggleSettingsDialog);
    navigate_to_setting(&mut state, "Read-Only");

    state.update(InputEvent::Input(' '));
    assert!(state.config.read_only, "space should toggle read_only on");
    assert!(
        matches!(state.open_dialog, Some(DialogState::Settings(_))),
        "space toggle should keep the dialog open"
    );

    state.update(InputEvent::Input(' '));
    assert!(
        !state.config.read_only,
        "second space should toggle read_only off"
    );
}

#[test]
fn settings_esc_closes() {
    let mut state = AppState::default();
    state.update(DialogEvent::ToggleSettingsDialog);
    state.update(ControlEvent::Abort);
    assert!(state.open_dialog.is_none());
}

#[test]
fn settings_select_toggles_steering_mode() {
    let mut state = AppState::default();
    state.update(DialogEvent::ToggleSettingsDialog);
    // Find steering mode row by scanning
    let count = settings_count(&state);
    assert!(matches!(
        state.config.steering_mode,
        DeliveryMode::OneAtATime
    ));
    for _ in 0..count {
        let is_steering = if let Some(DialogState::Settings(stack)) = &state.open_dialog {
            stack
                .current()
                .and_then(|p| p.selected_item())
                .and_then(|i| i.label())
                == Some("Steering Mode")
        } else {
            false
        };
        if is_steering {
            state.update(ModelConfigEvent::SettingsSelect);
            break;
        }
        state.update(ModelConfigEvent::SettingsDown);
    }
    assert!(matches!(state.config.steering_mode, DeliveryMode::All));
}

#[test]
fn settings_select_cycles_provider() {
    crate::tests::configure_test_providers(&[
        ("anthropic".into(), vec!["claude-3".into()]),
        ("openai".into(), vec!["gpt-4o".into()]),
    ]);
    let mut state = AppState::default();
    state.config.current_provider = "anthropic".into();
    state.update(DialogEvent::ToggleSettingsDialog);
    let count = settings_count(&state);
    for _ in 0..count {
        let is_provider = if let Some(DialogState::Settings(stack)) = &state.open_dialog {
            stack
                .current()
                .and_then(|p| p.selected_item())
                .and_then(|i| i.label())
                == Some("Provider")
        } else {
            false
        };
        if is_provider {
            state.update(ModelConfigEvent::SettingsSelect);
            break;
        }
        state.update(ModelConfigEvent::SettingsDown);
    }
    assert_eq!(state.config.current_provider, "openai");
}

#[test]
fn settings_select_cycles_theme() {
    let mut state = AppState::default();
    state.config.theme_name = "runie".into();
    state.update(DialogEvent::ToggleSettingsDialog);
    let count = settings_count(&state);
    for _ in 0..count {
        let is_theme = if let Some(DialogState::Settings(stack)) = &state.open_dialog {
            stack
                .current()
                .and_then(|p| p.selected_item())
                .and_then(|i| i.label())
                == Some("Theme")
        } else {
            false
        };
        if is_theme {
            state.update(ModelConfigEvent::SettingsSelect);
            break;
        }
        state.update(ModelConfigEvent::SettingsDown);
    }
    assert_ne!(state.config.theme_name, "runie");
}

// =========================================================================
// Coverage: every key in config.toml that is user-tunable must appear
// in the settings dialog.
// =========================================================================

fn has_item(state: &AppState, key: &str) -> bool {
    build_setting_items(state).iter().any(|i| i.key == key)
}

fn has_label(state: &AppState, label: &str) -> bool {
    build_setting_items(state).iter().any(|i| i.label == label)
}

#[allow(dead_code)]
fn find_index(state: &AppState, label: &str) -> Option<usize> {
    build_setting_items(state)
        .iter()
        .position(|i| i.label == label)
}

fn select_by_label(state: &mut AppState, label: &str) {
    let count = settings_count(state);
    for _ in 0..count {
        let is_match = if let Some(DialogState::Settings(stack)) = &state.open_dialog {
            stack
                .current()
                .and_then(|p| p.selected_item())
                .and_then(|i| i.label())
                == Some(label)
        } else {
            false
        };
        if is_match {
            state.update(ModelConfigEvent::SettingsSelect);
            return;
        }
        state.update(ModelConfigEvent::SettingsDown);
    }
    panic!("setting label {:?} not found", label);
}

fn navigate_to_setting(state: &mut AppState, label: &str) {
    let count = settings_count(state);
    for _ in 0..count {
        let is_match = if let Some(DialogState::Settings(stack)) = &state.open_dialog {
            stack
                .current()
                .and_then(|p| p.selected_item())
                .and_then(|i| i.label())
                == Some(label)
        } else {
            false
        };
        if is_match {
            return;
        }
        state.update(ModelConfigEvent::SettingsDown);
    }
    panic!("setting label {:?} not found", label);
}

#[test]
fn settings_includes_vim_mode_toggle() {
    let state = AppState::default();
    assert!(
        has_item(&state, "vim_mode"),
        "settings must expose vim_mode (from [ui] vim_mode)"
    );
    assert!(
        has_label(&state, "Vim Navigation"),
        "settings must have a 'Vim Navigation' row"
    );
}

#[test]
fn settings_vim_mode_default_is_true() {
    let state = AppState::default();
    let item = build_setting_items(&state)
        .into_iter()
        .find(|i| i.key == "vim_mode")
        .expect("vim_mode item");
    assert!(matches!(item.value, SettingValue::Bool(true)));
}

#[test]
fn settings_select_toggles_vim_mode() {
    let mut state = AppState::default();
    assert!(state.config.vim_mode);
    state.update(DialogEvent::ToggleSettingsDialog);
    select_by_label(&mut state, "Vim Navigation");
    assert!(!state.config.vim_mode, "select should turn vim_mode off");
    state.update(DialogEvent::ToggleSettingsDialog);
    select_by_label(&mut state, "Vim Navigation");
    assert!(state.config.vim_mode, "select should turn vim_mode on");
}

#[test]
fn settings_includes_telemetry_toggle() {
    let state = AppState::default();
    assert!(
        has_item(&state, "telemetry_enabled"),
        "settings must expose telemetry.enabled"
    );
}

#[test]
fn settings_select_toggles_telemetry() {
    let mut state = AppState::default();
    assert!(!state.config.telemetry.is_enabled());
    state.update(DialogEvent::ToggleSettingsDialog);
    select_by_label(&mut state, "Telemetry");
    assert!(
        state.config.telemetry.is_enabled(),
        "select should turn telemetry on"
    );
}

#[test]
fn settings_includes_truncation_fields() {
    let state = AppState::default();
    assert!(has_item(&state, "truncation_max_lines"));
    assert!(has_item(&state, "truncation_max_bytes"));
}

#[test]
fn settings_truncation_defaults_match_config() {
    let state = AppState::default();
    let lines_item = build_setting_items(&state)
        .into_iter()
        .find(|i| i.key == "truncation_max_lines")
        .expect("truncation_max_lines item");
    if let SettingValue::Cycle { current, options } = &lines_item.value {
        assert_eq!(current, "2000");
        assert!(options.iter().any(|o| o == "2000"));
    } else {
        panic!("truncation_max_lines should be Cycle");
    }
}

#[test]
fn settings_vim_mode_row_in_behavior_category() {
    let state = AppState::default();
    let item = build_setting_items(&state)
        .into_iter()
        .find(|i| i.key == "vim_mode")
        .expect("vim_mode item");
    assert!(matches!(item.category, SettingsCategory::Behavior));
}

#[test]
fn settings_contains_every_runtime_tunable_config_key() {
    // The settings dialog must expose every field that the user can change
    // at runtime and that comes from config.toml.
    let state = AppState::default();
    for key in [
        "vim_mode",
        "telemetry_enabled",
        "truncation_max_lines",
        "truncation_max_bytes",
    ] {
        assert!(
            has_item(&state, key),
            "settings must contain config key {key}"
        );
    }
}

#[test]
fn settings_select_changes_truncation_max_lines() {
    let mut state = AppState::default();
    let before = state.config.truncation.max_lines;
    state.update(DialogEvent::ToggleSettingsDialog);
    navigate_to_setting(&mut state, "Truncation Max Lines");
    state.update(ModelConfigEvent::SettingsSelect);

    assert_ne!(
        state.config.truncation.max_lines, before,
        "select should change truncation_max_lines"
    );
}

#[test]
fn settings_select_changes_truncation_max_bytes() {
    let mut state = AppState::default();
    let before = state.config.truncation.max_bytes;
    state.update(DialogEvent::ToggleSettingsDialog);
    navigate_to_setting(&mut state, "Truncation Max Bytes");
    state.update(ModelConfigEvent::SettingsSelect);

    assert_ne!(
        state.config.truncation.max_bytes, before,
        "select should change truncation_max_bytes"
    );
}

#[test]
fn settings_truncation_values_persist_after_close() {
    let mut state = AppState::default();
    state.update(DialogEvent::ToggleSettingsDialog);
    navigate_to_setting(&mut state, "Truncation Max Lines");
    state.update(ModelConfigEvent::SettingsSelect);
    let lines_after = state.config.truncation.max_lines;

    state.update(DialogEvent::ToggleSettingsDialog);
    navigate_to_setting(&mut state, "Truncation Max Bytes");
    state.update(ModelConfigEvent::SettingsSelect);
    let bytes_after = state.config.truncation.max_bytes;

    state.update(ControlEvent::Abort);
    assert!(state.open_dialog.is_none());
    assert_eq!(state.config.truncation.max_lines, lines_after);
    assert_eq!(state.config.truncation.max_bytes, bytes_after);
}
