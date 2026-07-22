//! Tests for the `/providers` command and login flow state machine.

use crate::dialog::PanelItem;
use crate::model::AppState;

use super::{add_provider_and_select_model, clean_config};

// ---------------------------------------------------------------------------
// Dialog / command entry tests
// ---------------------------------------------------------------------------

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
    crate::provider::config::save_provider_config(
        "openai",
        "https://api.openai.com/v1",
        "sk-test",
        &["gpt-4o".into()],
    )
    .unwrap();
    let mut state = AppState::default();
    state.update(crate::Event::ProvidersDialog);
    state.update(crate::Event::ProvidersEditModels { provider: "openai".into() });

    let stack = state
        .open_dialog
        .as_ref()
        .and_then(|d| d.panel_stack())
        .expect("panel stack should be open");
    let panel = stack.current().expect("editor panel should exist");
    assert_eq!(
        panel.id, "provider-models",
        "expected provider-models panel"
    );
    assert!(
        panel
            .items
            .iter()
            .any(|i| i.label().is_some_and(|l| l == "gpt-4o")),
        "editor should contain configured model"
    );
}

#[test]
fn edit_models_selects_active_mock_model() {
    clean_config();
    crate::provider::set_mock_enabled(true);
    let mut state = AppState::default();
    state.config_mut().current_provider = "mock".into();
    state.config_mut().current_model = "echo".into();

    state.update(crate::Event::ProvidersDialog);
    state.update(crate::Event::ProvidersEditModels { provider: "mock".into() });

    let panel = state
        .open_dialog
        .as_ref()
        .and_then(|d| d.panel_stack())
        .expect("panel stack should be open")
        .current()
        .expect("editor panel should exist");
    let echo_enabled = panel.items.iter().find_map(|i| match i {
        PanelItem::Toggle { label, value, .. } if label == "echo" => Some(*value),
        _ => None,
    });
    crate::provider::set_mock_enabled(false);
    assert_eq!(
        echo_enabled,
        Some(true),
        "active mock model echo should be selected in the provider models editor"
    );
}

// ---------------------------------------------------------------------------
// Login flow state machine tests
// ---------------------------------------------------------------------------

#[test]
fn login_flow_state_machine_provider_picker() {
    clean_config();
    let mut state = AppState::default();

    state.update(crate::Event::Start);

    assert!(state.login_flow.is_some());
    let flow = state.login_flow.as_ref().unwrap();
    assert_eq!(flow.step, crate::login_flow::LoginStep::ProviderPicker);
}

#[test]
fn login_flow_state_machine_key_input() {
    clean_config();
    let mut state = AppState::default();

    state.update(crate::Event::Start);
    state.update(crate::Event::SelectProvider { provider: "minimax".into() });

    let flow = state.login_flow.as_ref().unwrap();
    assert_eq!(flow.step, crate::login_flow::LoginStep::KeyInput);
    assert_eq!(flow.provider, "minimax");
}

#[test]
fn login_flow_state_machine_validating() {
    clean_config();
    let mut state = AppState::default();

    state.update(crate::Event::Start);
    state.update(crate::Event::SelectProvider { provider: "minimax".into() });
    state.update(crate::Event::SubmitKey { provider: "minimax".into(), key: "sk-test".into() });

    let flow = state.login_flow.as_ref().unwrap();
    assert_eq!(flow.step, crate::login_flow::LoginStep::Validating);
    assert_eq!(flow.key, "sk-test");
}

#[test]
fn login_flow_state_machine_model_select_after_validation() {
    clean_config();
    let mut state = AppState::default();

    state.update(crate::Event::Start);
    state.update(crate::Event::SelectProvider { provider: "minimax".into() });
    state.update(crate::Event::SubmitKey { provider: "minimax".into(), key: "sk-test".into() });
    state.update(crate::Event::ModelsFetched {
        provider: "minimax".into(),
        key: "sk-test".into(),
        models: vec!["MiniMax-M3".into()],
    });

    let flow = state.login_flow.as_ref().unwrap();
    assert_eq!(flow.step, crate::login_flow::LoginStep::ModelSelect);
    assert_eq!(flow.key, "sk-test");
}

#[test]
fn login_flow_toggle_model() {
    clean_config();
    let mut state = AppState::default();

    state.update(crate::Event::Start);
    state.update(crate::Event::SelectProvider { provider: "minimax".into() });
    state.update(crate::Event::SubmitKey { provider: "minimax".into(), key: "sk-test".into() });
    state.update(crate::Event::ModelsFetched {
        provider: "minimax".into(),
        key: "sk-test".into(),
        models: vec!["MiniMax-M3".into()],
    });

    let flow = state.login_flow.as_ref().unwrap();
    let initial_model = flow.available_models[0].clone();
    let was_selected = flow.selected_models.contains(&initial_model);

    state.update(crate::Event::ToggleModel { model: initial_model.clone() });

    let flow = state.login_flow.as_ref().unwrap();
    let is_selected = flow.selected_models.contains(&initial_model);
    assert_eq!(
        is_selected, !was_selected,
        "model selection should be toggled"
    );
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

#[test]
fn login_flow_with_unknown_provider() {
    clean_config();
    let mut state = AppState::default();

    state.update(crate::Event::Start);
    state.update(crate::Event::SelectProvider { provider: "unknown".into() });
    state.update(crate::Event::SubmitKey { provider: "unknown".into(), key: "sk-test".into() });

    let flow = state.login_flow.as_ref().unwrap();
    assert_eq!(flow.step, crate::login_flow::LoginStep::Validating);
    assert!(flow.available_models.is_empty());
    assert!(flow.selected_models.is_empty());
}

#[test]
fn providers_dialog_empty_state() {
    clean_config();
    let mut state = AppState::default();
    state.update(crate::Event::ProvidersDialog);

    assert!(state.open_dialog.is_some());
}

// ---------------------------------------------------------------------------
// Multiple providers
// ---------------------------------------------------------------------------

#[test]
fn disconnect_active_provider_switches_to_another() {
    clean_config();
    let mut state = AppState::default();

    add_provider_and_select_model(&mut state, "minimax", "sk-test", "MiniMax-M3");
    add_provider_and_select_model(&mut state, "openai", "sk-test-openai", "gpt-4o");

    assert_eq!(state.config.current_provider, "openai");

    state.dialog_back_stack.clear();

    state.update(crate::Event::ProvidersDialog);
    state.update(crate::Event::ProvidersDisconnect { provider: "openai".into() });

    assert_ne!(
        state.config.current_provider, "openai",
        "openai should not be current after disconnect"
    );
    assert!(
        state.open_dialog.is_none(),
        "dialog should be closed after disconnect"
    );
}
