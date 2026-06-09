//! Tests that global events (theme, model) pass through when dialogs are open.

use crate::model::AppState;
use crate::event::Event;
use crate::commands::DialogState;

#[test]
fn theme_switch_reaches_handler_while_settings_dialog_open() {
    let mut state = AppState::default();
    state.open_dialog = Some(DialogState::Settings {
        category: crate::settings::SettingsCategory::Models,
        selected: 0,
    });

    state.update(Event::SwitchTheme { name: "dracula".into() });

    assert_eq!(state.config.theme_name, "dracula");
    assert!(state.open_dialog.is_some(), "Dialog should remain open after theme switch");
}

#[test]
fn theme_switch_reaches_handler_while_palette_open() {
    let mut state = AppState::default();
    state.open_dialog = Some(DialogState::CommandPalette {
        filter: String::new(),
        selected: 0,
    });

    state.update(Event::SwitchTheme { name: "nord".into() });

    assert_eq!(state.config.theme_name, "nord");
    assert!(matches!(state.open_dialog, Some(DialogState::CommandPalette { .. })));
}

#[test]
fn model_switch_reaches_handler_while_dialog_open() {
    let mut state = AppState::default();
    state.open_dialog = Some(DialogState::Settings {
        category: crate::settings::SettingsCategory::Models,
        selected: 0,
    });

    state.update(Event::SwitchModel {
        provider: "openai".into(),
        model: "gpt-4o".into(),
    });

    assert_eq!(state.config.current_provider, "openai");
    assert_eq!(state.config.current_model, "gpt-4o");
    assert!(state.open_dialog.is_some());
}

#[test]
fn quit_works_while_dialog_open() {
    let mut state = AppState::default();
    state.open_dialog = Some(DialogState::CommandPalette {
        filter: String::new(),
        selected: 0,
    });

    state.update(Event::Quit);

    assert!(state.should_quit);
}
