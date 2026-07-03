#![allow(clippy::useless_conversion)]
//! End-to-end onboarding tests (Layer 2 + Layer 3).
//!
//! Drives the first-run login flow and the providers-add flow through core
//! events and verifies both state transitions and rendered UI.

use super::*;
use runie_core::login_flow::LoginStep;
use runie_core::Event;

fn clean_config() {
    let path = runie_core::provider::config::generate_test_config_path("runie_onboarding_e2e");
    let _ = std::fs::remove_file(&path);
    runie_core::provider::config::set_test_config_path(path);
}

#[test]
fn full_flow_from_empty_to_input_box() {
    clean_config();

    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    state.update(Event::Start);
    state.update(Event::SelectProvider {
        provider: "minimax".into(),
    });
    state.update(Event::SubmitKey {
        provider: "minimax".into(),
        key: "sk-test".into(),
    });
    state.update(Event::ModelsFetched {
        provider: "minimax".into(),
        key: "sk-test".into(),
        models: vec!["MiniMax-M3".into()],
    });
    state.update(Event::Save);

    assert!(state.has_models(), "model should be active after save");
    assert!(
        state.open_dialog.is_none(),
        "dialog should close after save"
    );
    assert_eq!(state.config.current_provider, "minimax");
    assert_eq!(state.config.current_model, "MiniMax-M3");

    let content = render_content(&mut state);
    assert!(
        content.contains(" minimax/MiniMax-M3 "),
        "input box title should appear after connecting, got: {}",
        content
    );
}

#[test]
fn validation_failure_shows_error() {
    clean_config();

    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    state.update(Event::Start);
    state.update(Event::SelectProvider {
        provider: "minimax".into(),
    });
    state.update(Event::SubmitKey {
        provider: "minimax".into(),
        key: "sk-test".into(),
    });
    state.update(Event::ValidationFailed {
        provider: "minimax".into(),
        key: "sk-test".into(),
        error: "invalid key".into(),
    });

    assert!(
        state.login_flow.is_some(),
        "flow should stay open after failure"
    );
    assert_eq!(
        state.login_flow.as_ref().unwrap().step,
        LoginStep::KeyInput,
        "should return to key input after validation failure"
    );

    let content = render_content(&mut state);
    assert!(
        content.contains("Could not verify key"),
        "error message should render, got: {}",
        content
    );
    assert!(content.contains("invalid key"));
}

#[test]
fn add_second_provider_keeps_first_active() {
    clean_config();
    configure_test_providers(&[("openai".into(), vec!["gpt-4o".into()])]);

    let mut state = AppState::default();
    state.config.current_provider = "openai".into();
    state.config.current_model = "gpt-4o".into();

    state.update(Event::ProvidersDialog);
    state.update(Event::ProvidersAdd);
    state.update(Event::SelectProvider {
        provider: "minimax".into(),
    });
    state.update(Event::SubmitKey {
        provider: "minimax".into(),
        key: "sk-test".into(),
    });
    state.update(Event::ModelsFetched {
        provider: "minimax".into(),
        key: "sk-test".into(),
        models: vec!["MiniMax-M3".into()],
    });
    state.update(Event::Save);

    let configured: Vec<String> = runie_core::provider::config::list_configured_providers()
        .into_iter()
        .map(|(name, _, _)| name)
        .collect();
    assert!(
        configured.contains(&"openai".into()),
        "openai should be saved"
    );
    assert!(
        configured.contains(&"minimax".into()),
        "minimax should be saved"
    );

    assert!(
        state.open_dialog.is_none(),
        "providers dialog should close after save"
    );
    assert_eq!(
        state.config.current_provider, "openai",
        "active provider should remain openai"
    );
    assert_eq!(
        state.config.current_model, "gpt-4o",
        "active model should remain gpt-4o"
    );

    let content = render_content(&mut state);
    assert!(
        content.contains(" openai/gpt-4o "),
        "input box title should keep first provider/model, got: {}",
        content
    );
}

#[test]
fn login_flow_auto_opens_when_no_model_connected() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    state.update(Event::Start);

    assert!(state.login_flow.is_some(), "login flow should auto-start");
    assert_eq!(
        state.login_flow.as_ref().unwrap().step,
        LoginStep::ProviderPicker,
        "should start at provider picker"
    );
    assert!(
        state.open_dialog.is_some(),
        "provider picker dialog should be open"
    );

    let content = render_content(&mut state);
    assert!(
        content.contains("Choose a provider"),
        "provider picker should render, got: {}",
        content
    );
}

#[test]
fn invalid_key_retry_with_valid_key_saves_and_connects() {
    clean_config();

    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    state.update(Event::Start);
    state.update(Event::SelectProvider {
        provider: "minimax".into(),
    });
    state.update(Event::SubmitKey {
        provider: "minimax".into(),
        key: "sk-bad".into(),
    });
    state.update(Event::ValidationFailed {
        provider: "minimax".into(),
        key: "sk-bad".into(),
        error: "invalid key".into(),
    });

    assert!(
        state.login_flow.is_some(),
        "flow should stay open after failure"
    );
    assert_eq!(
        state.login_flow.as_ref().unwrap().step,
        LoginStep::KeyInput,
        "should return to key input after validation failure"
    );
    let content = render_content(&mut state);
    assert!(
        content.contains("Could not verify key"),
        "error message should render, got: {}",
        content
    );
    assert!(content.contains("invalid key"));

    // Retry with a valid key.
    state.update(Event::SubmitKey {
        provider: "minimax".into(),
        key: "sk-good".into(),
    });
    state.update(Event::ModelsFetched {
        provider: "minimax".into(),
        key: "sk-good".into(),
        models: vec!["MiniMax-M3".into()],
    });

    assert_eq!(
        state.login_flow.as_ref().unwrap().step,
        LoginStep::ModelSelect,
        "should reach model select after retry success"
    );
    state.update(Event::Save);

    assert!(state.has_models(), "model should be active after save");
    assert!(
        state.open_dialog.is_none(),
        "dialog should close after save"
    );
    assert_eq!(state.config.current_provider, "minimax");
    assert_eq!(state.config.current_model, "MiniMax-M3");

    let content = render_content(&mut state);
    assert!(
        content.contains(" minimax/MiniMax-M3 "),
        "input box title should appear after connecting, got: {}",
        content
    );
}

#[test]
fn uncheck_all_models_rejects_save_with_transient_error() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    state.update(Event::Start);
    state.update(Event::SelectProvider {
        provider: "minimax".into(),
    });
    state.update(Event::SubmitKey {
        provider: "minimax".into(),
        key: "sk-test".into(),
    });
    state.update(Event::ModelsFetched {
        provider: "minimax".into(),
        key: "sk-test".into(),
        models: vec!["MiniMax-M3".into()],
    });

    assert_eq!(
        state.login_flow.as_ref().unwrap().step,
        LoginStep::ModelSelect
    );

    // Uncheck the only model.
    state.update(Event::from(Event::Input(' ')));
    assert!(
        !state
            .login_flow
            .as_ref()
            .unwrap()
            .selected_models
            .contains("MiniMax-M3"),
        "model should be unchecked"
    );

    // Move selection down to the _Save action and submit.
    state.update(Event::from(Event::HistoryNext));
    state.update(Event::from(Event::Submit));

    assert!(
        state.open_dialog.is_some(),
        "dialog should stay open when no model is selected"
    );
    let transient = state.transient_message.as_deref().unwrap_or("");
    assert!(
        transient.contains("Select at least one model"),
        "expected transient error about empty selection, got: {:?}",
        transient
    );
}
