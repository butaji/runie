use crate::Event;
use crate::model::AppState;

use super::clean_config;

/// Open palette and select a command by name
fn palette_select(state: &mut AppState, cmd: &str) {
    state.update(crate::Event::Input('/'));
    for c in cmd.chars() {
        state.update(crate::Event::PaletteFilter(c));
    }
    state.update(crate::Event::PaletteSelect);
}

#[test]
fn providers_command_opens_dialog() {
    clean_config();
    let mut state = AppState::default();
    state.update(crate::Event::ProvidersDialog);
    assert!(
        state.open_dialog.is_some(),
        "/providers must open the dialog"
    );
}

#[test]
fn slash_providers_opens_dialog() {
    clean_config();
    let mut state = AppState::default();
    palette_select(&mut state, "providers");

    assert!(
        state.open_dialog.is_some(),
        "raw /providers command should open the dialog"
    );
}

#[test]
fn slash_provider_alias_opens_dialog() {
    clean_config();
    let mut state = AppState::default();
    palette_select(&mut state, "provider");

    assert!(
        state.open_dialog.is_some(),
        "raw /provider command should open the dialog"
    );
}

#[test]
fn edit_models_opens_dedicated_panel() {
    clean_config();
    crate::login_config::save_provider_config(
        "openai",
        "https://api.openai.com/v1",
        "sk-test",
        &["gpt-4o".into()],
    )
    .unwrap();
    let mut state = AppState::default();
    state.update(crate::Event::ProvidersDialog);
    state.update(crate::Event::ProvidersEditModels {
        provider: "openai".into(),
    });

    let stack = state
        .open_dialog
        .as_ref()
        .and_then(|d| d.panel_stack())
        .expect("panel stack should be open");
    let panel = stack.current().expect("editor panel should exist");
    assert_eq!(panel.id, "provider-models", "expected provider-models panel");
    assert!(
        panel.items.iter().any(|i| i.label().is_some_and(|l| l == "gpt-4o")),
        "editor should contain configured model"
    );
}
