//! Settings dialog tests (Layer 2 + Layer 3)

use crate::commands::DialogState;
use crate::event::Event;
use crate::model::{AppState, DeliveryMode};
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

#[test]
fn settings_opens_dialog() {
    let mut state = AppState::default();
    for c in "/settings".chars() {
        state.update(Event::Input(c));
    }
    state.update(Event::Submit);
    assert!(
        matches!(state.open_dialog, Some(DialogState::Settings(_))),
        "Expected Settings dialog, got {:?}",
        state.open_dialog
    );
}

#[test]
fn settings_navigates_up() {
    let mut state = AppState::default();
    state.update(Event::ToggleSettingsDialog);
    state.update(Event::SettingsUp);
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
    state.update(Event::ToggleSettingsDialog);
    let count = settings_count(&state);
    for _ in 0..count {
        state.update(Event::SettingsDown);
    }
    let selected = settings_selected(&state).expect("Dialog should still be open");
    assert_eq!(selected, 0, "Down wraps to first");
}

#[test]
fn settings_select_toggles_read_only() {
    let mut state = AppState::default();
    state.config.read_only = false;
    state.update(Event::ToggleSettingsDialog);
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
            state.update(Event::SettingsSelect);
            break;
        }
        state.update(Event::SettingsDown);
    }
    assert!(state.config.read_only);
}

#[test]
fn settings_esc_closes() {
    let mut state = AppState::default();
    state.update(Event::ToggleSettingsDialog);
    state.update(Event::Abort);
    assert!(state.open_dialog.is_none());
}

#[test]
fn settings_select_toggles_steering_mode() {
    let mut state = AppState::default();
    state.update(Event::ToggleSettingsDialog);
    // Find steering mode row by scanning
    let count = settings_count(&state);
    assert!(matches!(state.config.steering_mode, DeliveryMode::OneAtATime));
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
            state.update(Event::SettingsSelect);
            break;
        }
        state.update(Event::SettingsDown);
    }
    assert!(matches!(state.config.steering_mode, DeliveryMode::All));
}

#[test]
fn settings_select_cycles_provider() {
    let mut state = AppState::default();
    state.config.current_provider = "anthropic".into();
    state.update(Event::ToggleSettingsDialog);
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
            state.update(Event::SettingsSelect);
            break;
        }
        state.update(Event::SettingsDown);
    }
    assert_eq!(state.config.current_provider, "openai");
}

#[test]
fn settings_select_cycles_theme() {
    let mut state = AppState::default();
    state.config.theme_name = "runie".into();
    state.update(Event::ToggleSettingsDialog);
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
            state.update(Event::SettingsSelect);
            break;
        }
        state.update(Event::SettingsDown);
    }
    assert_ne!(state.config.theme_name, "runie");
}
