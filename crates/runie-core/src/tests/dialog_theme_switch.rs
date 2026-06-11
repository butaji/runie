//! Tests that global events (theme, model) pass through when dialogs are open.

use crate::model::AppState;
use crate::event::Event;
use crate::commands::DialogState;
use crate::dialog::builders::theme_picker;

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

/// Layer 1 test: Theme picker panel has keep_open_on_activate enabled.
/// This enables live theme preview - Enter applies theme but dialog stays open.
#[test]
fn theme_picker_panel_keeps_open_for_preview() {
    let stack = theme_picker(vec![
        ("runie".into(), Event::SwitchTheme { name: "runie".into() }),
        ("dracula".into(), Event::SwitchTheme { name: "dracula".into() }),
    ]);
    let panel = stack.current().expect("panel stack should have a panel");

    // Theme picker must have keep_open flag so Enter applies theme but keeps dialog open
    assert!(
        panel.keep_open_on_activate,
        "Theme picker should have keep_open_on_activate for live preview"
    );

    // Verify the Emit action will be sent
    assert!(panel.items.iter().any(|item| {
        matches!(item, crate::dialog::PanelItem::Action { action: crate::dialog::ItemAction::Emit(_), .. })
    }));
}
