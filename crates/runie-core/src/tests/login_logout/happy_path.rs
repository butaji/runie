//! Onboarding happy path tests.

use crate::login_flow::LoginStep;
use crate::model::AppState;
use crate::Event;

use super::{
    assert_panel_id, assert_step, clean_config, fetch_models, save_login_flow, select_provider,
    start_login_flow, submit_key,
};

#[test]
fn auto_open_starts_provider_picker_when_no_provider() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    start_login_flow(&mut state);

    assert_step(&state, LoginStep::ProviderPicker);
    assert_panel_id(&state, "login-provider");
}

#[test]
fn full_happy_path_connects_provider() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    start_login_flow(&mut state);
    select_provider(&mut state, "minimax");
    submit_key(&mut state, "sk-test");
    fetch_models(&mut state, &["MiniMax-M3".into()]);
    save_login_flow(&mut state);

    assert!(
        state.has_models(),
        "provider should be connected after save"
    );
    assert_eq!(state.config.current_provider, "minimax");
    assert_eq!(state.config.current_model, "MiniMax-M3");
    assert!(state.login_flow.is_none(), "login flow should be cleared");
    assert!(state.open_dialog.is_none(), "dialog should be closed");
}

#[test]
fn save_activates_first_selected_model() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    start_login_flow(&mut state);
    select_provider(&mut state, "minimax");
    submit_key(&mut state, "sk-test");
    fetch_models(&mut state, &["MiniMax-M3".into(), "MiniMax-M2".into()]);
    save_login_flow(&mut state);

    assert_eq!(state.config.current_provider, "minimax");
    assert_eq!(
        state.config.current_model, "MiniMax-M3",
        "first model in available_models order should be activated"
    );
}

#[test]
fn validation_done_legacy_reaches_model_select() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    start_login_flow(&mut state);
    select_provider(&mut state, "minimax");
    submit_key(&mut state, "sk-test");
    state.update(crate::Event::ModelsFetched {
        provider: "minimax".into(),
        key: "sk-test".into(),
        models: vec!["MiniMax-M3".into()],
    });

    assert_step(&state, LoginStep::ModelSelect);
}

#[test]
fn start_with_existing_model_does_not_auto_open() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider = "mock".into();
    state.config.current_model = "echo".into();

    assert!(state.has_models(), "test setup should have a model");
    assert!(
        state.login_flow.is_none(),
        "login flow should not open automatically when a model is connected"
    );

    // An explicit Start still opens the provider picker, but it is closable.
    start_login_flow(&mut state);
    assert_step(&state, LoginStep::ProviderPicker);
    assert_panel_id(&state, "login-provider");

    state.update(crate::Event::Abort);
    assert!(
        state.login_flow.is_none(),
        "picker should close because a model is already connected"
    );
}

#[tokio::test]
async fn model_selector_reflects_login_selected_models_under_runtime() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    start_login_flow(&mut state);
    select_provider(&mut state, "minimax");
    submit_key(&mut state, "sk-test");
    fetch_models(&mut state, &["MiniMax-M3".into(), "MiniMax-M2.7".into()]);
    state.update(Event::ToggleModel {
        model: "MiniMax-M2.7".into(),
    });
    save_login_flow(&mut state);

    state.update(Event::ToggleModelSelector);

    let dialog = state.open_dialog.expect("model selector should be open");
    let stack = dialog.panel_stack().expect("panel stack");
    let panel = stack.current().expect("panel");
    let model_labels: Vec<&str> = panel
        .items
        .iter()
        .filter_map(|i| match i {
            crate::dialog::PanelItem::Action { label, .. } => Some(label.as_str()),
            _ => None,
        })
        .collect();

    assert!(
        model_labels.iter().any(|l| l.contains("MiniMax-M3")),
        "selector should include chosen MiniMax-M3, got {:?}",
        model_labels
    );
    assert!(
        !model_labels.iter().any(|l| l.contains("MiniMax-M2.7")),
        "selector should not include deselected MiniMax-M2.7, got {:?}",
        model_labels
    );
}

#[test]
fn model_selector_reflects_login_selected_models() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    start_login_flow(&mut state);
    select_provider(&mut state, "minimax");
    submit_key(&mut state, "sk-test");
    fetch_models(&mut state, &["MiniMax-M3".into(), "MiniMax-M2.7".into()]);

    // Deselect MiniMax-M2.7 so only MiniMax-M3 remains chosen.
    state.update(crate::Event::ToggleModel {
        model: "MiniMax-M2.7".into(),
    });
    save_login_flow(&mut state);

    // Open the /model selector and inspect its items.
    state.update(Event::ToggleModelSelector);

    let dialog = state.open_dialog.expect("model selector should be open");
    let stack = dialog.panel_stack().expect("panel stack");
    let panel = stack.current().expect("panel");
    let model_labels: Vec<&str> = panel
        .items
        .iter()
        .filter_map(|i| match i {
            crate::dialog::PanelItem::Action { label, .. } => Some(label.as_str()),
            _ => None,
        })
        .collect();

    assert!(
        model_labels.iter().any(|l| l.contains("MiniMax-M3")),
        "selector should include chosen MiniMax-M3, got {:?}",
        model_labels
    );
    assert!(
        !model_labels.iter().any(|l| l.contains("MiniMax-M2.7")),
        "selector should not include deselected MiniMax-M2.7, got {:?}",
        model_labels
    );
}

#[test]
fn model_selector_reflects_ui_toggled_login_models() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    start_login_flow(&mut state);
    select_provider(&mut state, "minimax");
    submit_key(&mut state, "sk-test");
    fetch_models(&mut state, &["MiniMax-M3".into(), "MiniMax-M2.7".into()]);

    // The model selector panel is a form. Navigate down to MiniMax-M2.7
    // (first item is MiniMax-M3, second is MiniMax-M2.7) and toggle it off
    // with Space, then move to _Save and submit.
    state.update(crate::Event::CommandFormDown);
    state.update(crate::Event::Input(' ').into());
    state.update(crate::Event::CommandFormDown);
    state.update(crate::Event::Submit.into());

    // Open the /model selector and inspect its items.
    state.update(crate::Event::ToggleModelSelector);

    let dialog = state.open_dialog.expect("model selector should be open");
    let stack = dialog.panel_stack().expect("panel stack");
    let panel = stack.current().expect("panel");
    let model_labels: Vec<&str> = panel
        .items
        .iter()
        .filter_map(|i| match i {
            crate::dialog::PanelItem::Action { label, .. } => Some(label.as_str()),
            _ => None,
        })
        .collect();

    assert!(
        model_labels.iter().any(|l| l.contains("MiniMax-M3")),
        "selector should include kept MiniMax-M3, got {:?}",
        model_labels
    );
    assert!(
        !model_labels.iter().any(|l| l.contains("MiniMax-M2.7")),
        "selector should not include toggled-off MiniMax-M2.7, got {:?}",
        model_labels
    );
}
