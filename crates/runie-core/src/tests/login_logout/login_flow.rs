use crate::event::{DialogEvent, LoginFlowEvent};
use crate::login_config::list_configured_providers;
use crate::model::AppState;

use super::{clean_config, default_models_for_provider, validate_provider};

#[test]
fn login_flow_save_requires_validation() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    state.update(DialogEvent::ProvidersDialog);
    state.update(DialogEvent::ProvidersAdd);
    state.update(LoginFlowEvent::SelectProvider {
        provider: "minimax".into(),
    });
    state.update(LoginFlowEvent::SubmitKey {
        provider: "minimax".into(),
        key: "sk-test".into(),
    });
    state.update(LoginFlowEvent::Save);

    assert!(
        state.login_flow.is_some(),
        "save should be rejected without validation"
    );
    assert!(
        list_configured_providers().is_empty(),
        "provider should not be saved without validation"
    );
}

#[test]
fn login_flow_save_activates_first_model_after_validation() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    state.update(DialogEvent::ProvidersDialog);
    state.update(DialogEvent::ProvidersAdd);
    state.update(LoginFlowEvent::SelectProvider {
        provider: "minimax".into(),
    });
    state.update(LoginFlowEvent::SubmitKey {
        provider: "minimax".into(),
        key: "sk-test".into(),
    });
    validate_provider(&mut state, "minimax", "sk-test");
    state.update(LoginFlowEvent::Save);

    assert!(
        state.login_flow.is_none(),
        "login flow should be cleared after save"
    );
    assert!(
        state.open_dialog.is_none(),
        "dialog should be closed after save"
    );
    assert_eq!(state.config.current_provider, "minimax");
    assert!(
        !state.config.current_model.is_empty(),
        "a model should be auto-activated"
    );
}

#[test]
fn login_flow_save_saves_config() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    state.update(DialogEvent::ProvidersDialog);
    state.update(DialogEvent::ProvidersAdd);
    state.update(LoginFlowEvent::SelectProvider {
        provider: "minimax".into(),
    });
    state.update(LoginFlowEvent::SubmitKey {
        provider: "minimax".into(),
        key: "sk-test".into(),
    });
    validate_provider(&mut state, "minimax", "sk-test");
    state.update(LoginFlowEvent::Save);

    let configured = list_configured_providers();
    assert!(
        configured.iter().any(|(n, _, _)| n == "minimax"),
        "provider should be saved to config.toml"
    );
}

#[test]
fn login_flow_save_allows_model_selection_after_auto_activation() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    state.update(DialogEvent::ProvidersDialog);
    state.update(DialogEvent::ProvidersAdd);
    state.update(LoginFlowEvent::SelectProvider {
        provider: "minimax".into(),
    });
    state.update(LoginFlowEvent::SubmitKey {
        provider: "minimax".into(),
        key: "sk-test".into(),
    });
    validate_provider(&mut state, "minimax", "sk-test");
    state.update(LoginFlowEvent::Save);

    state.update(DialogEvent::ProvidersSelectModel {
        provider: "minimax".into(),
        model: "MiniMax-M3".into(),
    });

    assert_eq!(state.config.current_provider, "minimax");
    assert_eq!(state.config.current_model, "MiniMax-M3");
}

#[test]
fn login_flow_save_allows_model_selection_from_multiple() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    state.update(DialogEvent::ProvidersDialog);
    state.update(DialogEvent::ProvidersAdd);
    state.update(LoginFlowEvent::SelectProvider {
        provider: "openai".into(),
    });
    state.update(LoginFlowEvent::SubmitKey {
        provider: "openai".into(),
        key: "sk-test".into(),
    });

    let defaults = default_models_for_provider("openai");
    validate_provider(&mut state, "openai", "sk-test");
    state.update(LoginFlowEvent::Save);

    if defaults.len() >= 2 {
        state.update(DialogEvent::ProvidersSelectModel {
            provider: "openai".into(),
            model: defaults[1].to_string(),
        });
    }

    assert_eq!(state.config.current_provider, "openai");
    if defaults.len() >= 2 {
        assert_eq!(state.config.current_model, defaults[1]);
    }
}
