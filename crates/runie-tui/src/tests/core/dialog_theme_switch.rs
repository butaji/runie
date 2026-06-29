//! Tests that global events (theme, model) pass through when dialogs are open.

use runie_core::Event;

use runie_core::commands::{DialogKind, DialogState};
use runie_core::dialog::builders::theme_picker;
use runie_core::model::AppState;

#[test]
fn theme_switch_reaches_handler_while_settings_dialog_open() {
    let mut state = AppState::default();
    state.update(Event::ToggleSettingsDialog);

    state.update(Event::SwitchTheme {
        name: "dracula".into(),
    });

    assert_eq!(state.config.theme_name, "dracula");
    assert!(
        matches!(state.open_dialog, Some(DialogState::Active { kind: DialogKind::Settings, panels: _ })),
        "Dialog should remain open after theme switch"
    );
}

#[test]
fn theme_switch_reaches_handler_while_palette_open() {
    let mut state = AppState::default();
    state.update(Event::ToggleCommandPalette);

    state.update(Event::SwitchTheme {
        name: "nord".into(),
    });

    assert_eq!(state.config.theme_name, "nord");
    assert!(matches!(
        state.open_dialog,
        Some(DialogState::Active { kind: DialogKind::CommandPalette, panels: _ })
    ));
}

#[test]
fn model_switch_reaches_handler_while_dialog_open() {
    let mut state = AppState::default();
    state.update(Event::ToggleSettingsDialog);

    state.update(Event::SwitchModel {
        provider: "openai".into(),
        model: "gpt-4o".into(),
        explicit: true,
    });

    assert_eq!(state.config.current_provider, "openai");
    assert_eq!(state.config.current_model, "gpt-4o");
    assert!(matches!(state.open_dialog, Some(DialogState::Active { kind: DialogKind::Settings, panels: _ })));
}

#[test]
fn quit_works_while_dialog_open() {
    let mut state = AppState::default();
    state.update(Event::ToggleCommandPalette);

    state.update(Event::Quit);

    assert!(state.should_quit);
}

/// Layer 1 test: Theme picker panel has keep_open_on_activate enabled.
/// This enables live theme preview - Enter applies theme but dialog stays open.
#[test]
fn theme_picker_panel_keeps_open_for_preview() {
    let stack = theme_picker(vec![
        (
            "runie".into(),
            Event::SwitchTheme {
                name: "runie".into(),
            },
        ),
        (
            "dracula".into(),
            Event::SwitchTheme {
                name: "dracula".into(),
            },
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
            runie_core::dialog::PanelItem::Action {
                action: runie_core::dialog::ItemAction::Emit(_),
                ..
            }
        )
    }));
}

#[test]
fn theme_picker_activation_switches_theme() {
    let mut state = AppState::default();
    state.update(Event::ToggleCommandPalette);
    // Simulate selecting /theme from palette and opening the picker.
    let stack = theme_picker(vec![
        (
            "runie".into(),
            Event::SwitchTheme {
                name: "runie".into(),
            },
        ),
        (
            "dracula".into(),
            Event::SwitchTheme {
                name: "dracula".into(),
            },
        ),
    ]);
    state.open_dialog = Some(DialogState::Active { kind: DialogKind::Generic, panels: stack });
    state.update(Event::HistoryNext);
    state.update(Event::Submit);

    assert_eq!(state.config.theme_name, "dracula");
    assert!(
        matches!(state.open_dialog, Some(DialogState::Active { kind: DialogKind::Generic, panels: _ })),
        "Theme picker should stay open after applying theme"
    );
}

#[test]
fn theme_picker_filter_and_submit_switches_theme() {
    let mut state = AppState::default();
    let stack = theme_picker(vec![
        (
            "runie".into(),
            Event::SwitchTheme {
                name: "runie".into(),
            },
        ),
        (
            "dracula".into(),
            Event::SwitchTheme {
                name: "dracula".into(),
            },
        ),
    ]);
    state.open_dialog = Some(DialogState::Active { kind: DialogKind::Generic, panels: stack });
    for c in "dracula".chars() {
        state.update(Event::Input(c));
    }
    state.update(Event::Submit);

    assert_eq!(state.config.theme_name, "dracula");
}
