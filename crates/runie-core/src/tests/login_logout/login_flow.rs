use crate::event::{DialogEvent, LoginFlowEvent};
use crate::login_config::list_configured_providers;
use crate::model::AppState;

use super::clean_config;

#[test]
fn login_flow_save_shows_providers_dialog() {
    clean_config();
    let mut state = AppState::default();

    state.update(DialogEvent::ProvidersDialog);
    assert!(state.open_dialog.is_some());

    state.update(DialogEvent::ProvidersAdd);
    assert!(state.login_flow.is_some());

    state.update(LoginFlowEvent::SelectProvider {
        provider: "minimax".into(),
    });
    state.update(LoginFlowEvent::SubmitKey {
        provider: "minimax".into(),
        key: "sk-test".into(),
    });
    state.update(LoginFlowEvent::Save);

    assert!(
        state.open_dialog.is_some(),
        "providers dialog should be shown after login flow save"
    );
    assert!(state.login_flow.is_none(), "login flow should be cleared");
}

#[test]
fn login_flow_save_does_not_auto_activate_model() {
    clean_config();
    let mut state = AppState::default();

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
        state.config.current_provider.is_empty(),
        "provider should not be auto-activated after save"
    );
    assert!(
        state.config.current_model.is_empty(),
        "model should not be auto-activated after save"
    );
}

#[test]
fn login_flow_save_allows_model_selection() {
    clean_config();
    let mut state = AppState::default();

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

    state.update(DialogEvent::ProvidersDialog);
    state.update(DialogEvent::ProvidersAdd);
    state.update(LoginFlowEvent::SelectProvider {
        provider: "openai".into(),
    });
    state.update(LoginFlowEvent::SubmitKey {
        provider: "openai".into(),
        key: "sk-test".into(),
    });

    let defaults: Vec<String> = crate::provider_registry::find_provider("openai")
        .map(|m| {
            m.models
                .iter()
                .map(|model| model.name.to_string())
                .collect()
        })
        .unwrap_or_default();

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

#[test]
fn login_flow_save_saves_config() {
    clean_config();
    let mut state = AppState::default();

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

    let configured = list_configured_providers();
    assert!(
        configured.iter().any(|(n, _, _)| n == "minimax"),
        "provider should be saved to config.toml"
    );
}
