use crate::Event;
use crate::model::AppState;

use super::clean_config;

#[test]
fn login_flow_with_unknown_provider() {
    clean_config();
    let mut state = AppState::default();

    state.update(crate::Event::Start);
    state.update(crate::Event::SelectProvider {
        provider: "unknown".into(),
    });
    state.update(crate::Event::SubmitKey {
        provider: "unknown".into(),
        key: "sk-test".into(),
    });

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
