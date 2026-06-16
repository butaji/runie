use crate::event::{Event, LoginFlowEvent};
use crate::model::AppState;

use super::clean_config;

#[test]
fn login_flow_state_machine_provider_picker() {
    clean_config();
    let mut state = AppState::default();

    state.update(Event::LoginFlow(LoginFlowEvent::Start));

    assert!(state.login_flow.is_some());
    let flow = state.login_flow.as_ref().unwrap();
    assert_eq!(flow.step, crate::login_flow::LoginStep::ProviderPicker);
}

#[test]
fn login_flow_state_machine_key_input() {
    clean_config();
    let mut state = AppState::default();

    state.update(Event::LoginFlow(LoginFlowEvent::Start));
    state.update(Event::LoginFlow(LoginFlowEvent::SelectProvider {
        provider: "minimax".into(),
    }));

    let flow = state.login_flow.as_ref().unwrap();
    assert_eq!(flow.step, crate::login_flow::LoginStep::KeyInput);
    assert_eq!(flow.provider, "minimax");
}

#[test]
fn login_flow_state_machine_model_select() {
    clean_config();
    let mut state = AppState::default();

    state.update(Event::LoginFlow(LoginFlowEvent::Start));
    state.update(Event::LoginFlow(LoginFlowEvent::SelectProvider {
        provider: "minimax".into(),
    }));
    state.update(Event::LoginFlow(LoginFlowEvent::SubmitKey {
        provider: "minimax".into(),
        key: "sk-test".into(),
    }));

    let flow = state.login_flow.as_ref().unwrap();
    assert_eq!(flow.step, crate::login_flow::LoginStep::ModelSelect);
    assert_eq!(flow.key, "sk-test");
}

#[test]
fn login_flow_toggle_model() {
    clean_config();
    let mut state = AppState::default();

    state.update(Event::LoginFlow(LoginFlowEvent::Start));
    state.update(Event::LoginFlow(LoginFlowEvent::SelectProvider {
        provider: "minimax".into(),
    }));
    state.update(Event::LoginFlow(LoginFlowEvent::SubmitKey {
        provider: "minimax".into(),
        key: "sk-test".into(),
    }));

    let flow = state.login_flow.as_ref().unwrap();
    let initial_model = flow.available_models[0].clone();
    let was_selected = flow.selected_models.contains(&initial_model);

    state.update(Event::LoginFlow(LoginFlowEvent::ToggleModel {
        model: initial_model.clone(),
    }));

    let flow = state.login_flow.as_ref().unwrap();
    let is_selected = flow.selected_models.contains(&initial_model);
    assert_eq!(
        is_selected, !was_selected,
        "model selection should be toggled"
    );
}
