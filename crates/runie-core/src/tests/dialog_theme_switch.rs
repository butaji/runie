//! Tests that global events (theme, model) pass through when dialogs are open.

use crate::event::{ControlEvent, DialogEvent, Event, ModelConfigEvent};

use crate::commands::DialogState;
use crate::dialog::builders::theme_picker;
use crate::model::AppState;

#[test]
fn theme_switch_reaches_handler_while_settings_dialog_open() {
    let mut state = AppState::default();
    state.update(Event::ModelConfig(ModelConfigEvent::ToggleSettingsDialog));

    state.update(Event::ModelConfig(ModelConfigEvent::SwitchTheme {
        name: "dracula".into(),
    }));

    assert_eq!(state.config.theme_name, "dracula");
    assert!(
        matches!(state.open_dialog, Some(DialogState::Settings(_))),
        "Dialog should remain open after theme switch"
    );
}

#[test]
fn theme_switch_reaches_handler_while_palette_open() {
    let mut state = AppState::default();
    state.update(Event::Dialog(DialogEvent::ToggleCommandPalette));

    state.update(Event::ModelConfig(ModelConfigEvent::SwitchTheme {
        name: "nord".into(),
    }));

    assert_eq!(state.config.theme_name, "nord");
    assert!(matches!(
        state.open_dialog,
        Some(DialogState::CommandPalette(_))
    ));
}

#[test]
fn model_switch_reaches_handler_while_dialog_open() {
    let mut state = AppState::default();
    state.update(Event::ModelConfig(ModelConfigEvent::ToggleSettingsDialog));

    state.update(Event::ModelConfig(ModelConfigEvent::SwitchModel {
        provider: "openai".into(),
        model: "gpt-4o".into(),
    }));

    assert_eq!(state.config.current_provider, "openai");
    assert_eq!(state.config.current_model, "gpt-4o");
    assert!(matches!(state.open_dialog, Some(DialogState::Settings(_))));
}

#[test]
fn quit_works_while_dialog_open() {
    let mut state = AppState::default();
    state.update(Event::Dialog(DialogEvent::ToggleCommandPalette));

    state.update(Event::Control(ControlEvent::Quit));

    assert!(state.should_quit);
}

/// Layer 1 test: Theme picker panel has keep_open_on_activate enabled.
/// This enables live theme preview - Enter applies theme but dialog stays open.
#[test]
fn theme_picker_panel_keeps_open_for_preview() {
    let stack = theme_picker(vec![
        (
            "runie".into(),
            Event::ModelConfig(ModelConfigEvent::SwitchTheme {
                name: "runie".into(),
            }),
        ),
        (
            "dracula".into(),
            Event::ModelConfig(ModelConfigEvent::SwitchTheme {
                name: "dracula".into(),
            }),
        ),
    ]);
    let panel = stack.current().expect("panel stack should have a panel");

    // Theme picker must have keep_open flag so Enter applies theme but keeps dialog open
    assert!(
        panel.keep_open_on_activate,
        "Theme picker should have keep_open_on_activate for live preview"
    );

    // Verify the Emit action will be sent
    assert!(panel.items.iter().any(|item| {
        matches!(
            item,
            crate::dialog::PanelItem::Action {
                action: crate::dialog::ItemAction::Emit(_),
                ..
            }
        )
    }));
}
