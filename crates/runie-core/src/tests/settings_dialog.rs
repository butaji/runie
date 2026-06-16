//! Settings dialog tests (Layer 2 + Layer 3)

use crate::commands::DialogState;
use crate::event::{ControlEvent, Event, InputEvent, ModelConfigEvent, DialogEvent};
use crate::model::{AppState, DeliveryMode};
use crate::settings::{SettingValue, SettingsCategory};
use crate::update::settings_dialog::build_setting_items;

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
    state.update(Event::Input(InputEvent::Input('/')));
    for c in cmd.chars() {
        state.update(Event::Dialog(DialogEvent::PaletteFilter(c)));
    }
    state.update(Event::Dialog(DialogEvent::PaletteSelect));
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
    state.update(Event::Dialog(DialogEvent::ToggleSettingsDialog));
    state.update(Event::ModelConfig(ModelConfigEvent::SettingsUp));
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
    state.update(Event::Dialog(DialogEvent::ToggleSettingsDialog));
    let count = settings_count(&state);
    for _ in 0..count {
        state.update(Event::ModelConfig(ModelConfigEvent::SettingsDown));
    }
    let selected = settings_selected(&state).expect("Dialog should still be open");
    assert_eq!(selected, 0, "Down wraps to first");
}

#[test]
fn settings_select_toggles_read_only() {
    let mut state = AppState::default();
    state.config.read_only = false;
    state.update(Event::Dialog(DialogEvent::ToggleSettingsDialog));
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
            state.update(Event::ModelConfig(ModelConfigEvent::SettingsSelect));
            break;
        }
        state.update(Event::ModelConfig(ModelConfigEvent::SettingsDown));
    }
    assert!(state.config.read_only);
}

#[test]
fn settings_esc_closes() {
    let mut state = AppState::default();
    state.update(Event::Dialog(DialogEvent::ToggleSettingsDialog));
    state.update(Event::Control(ControlEvent::Abort));
    assert!(state.open_dialog.is_none());
}

#[test]
fn settings_select_toggles_steering_mode() {
    let mut state = AppState::default();
    state.update(Event::Dialog(DialogEvent::ToggleSettingsDialog));
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
            state.update(Event::ModelConfig(ModelConfigEvent::SettingsSelect));
            break;
        }
        state.update(Event::ModelConfig(ModelConfigEvent::SettingsDown));
    }
    assert!(matches!(state.config.steering_mode, DeliveryMode::All));
}

#[test]
fn settings_select_cycles_provider() {
    let mut state = AppState::default();
    state.config.current_provider = "anthropic".into();
    state.update(Event::Dialog(DialogEvent::ToggleSettingsDialog));
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
            state.update(Event::ModelConfig(ModelConfigEvent::SettingsSelect));
            break;
        }
        state.update(Event::ModelConfig(ModelConfigEvent::SettingsDown));
    }
    assert_eq!(state.config.current_provider, "openai");
}

#[test]
fn settings_select_cycles_theme() {
    let mut state = AppState::default();
    state.config.theme_name = "runie".into();
    state.update(Event::Dialog(DialogEvent::ToggleSettingsDialog));
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
            state.update(Event::ModelConfig(ModelConfigEvent::SettingsSelect));
            break;
        }
        state.update(Event::ModelConfig(ModelConfigEvent::SettingsDown));
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
            state.update(Event::ModelConfig(ModelConfigEvent::SettingsSelect));
            return;
        }
        state.update(Event::ModelConfig(ModelConfigEvent::SettingsDown));
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
    state.update(Event::Dialog(DialogEvent::ToggleSettingsDialog));
    select_by_label(&mut state, "Vim Navigation");
    assert!(!state.config.vim_mode, "select should turn vim_mode off");
    state.update(Event::Dialog(DialogEvent::ToggleSettingsDialog));
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
    state.update(Event::Dialog(DialogEvent::ToggleSettingsDialog));
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
    if let SettingValue::Enum { current, options } = &lines_item.value {
        assert_eq!(current, "2000");
        assert!(options.iter().any(|o| o == "2000"));
    } else {
        panic!("truncation_max_lines should be Enum");
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
