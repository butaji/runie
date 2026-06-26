use crate::Event;
use crate::login_config::list_configured_providers;
use crate::model::AppState;

use crate::tests::login_logout::clean_config;
use crate::tests::login_logout::default_models_for_provider;
use crate::tests::login_logout::validate_provider;

#[test]
fn providers_dialog_down_navigation_moves_selection() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    state.update(crate::Event::ProvidersDialog);
    let stack = state
        .open_dialog
        .as_ref()
        .and_then(|d| d.panel_stack())
        .expect("providers dialog should be open");
    let first_selected = stack.current().unwrap().selected;

    state.update(crate::Event::HistoryNext);

    let stack = state
        .open_dialog
        .as_ref()
        .and_then(|d| d.panel_stack())
        .expect("providers dialog should stay open");
    assert_ne!(
        stack.current().unwrap().selected,
        first_selected,
        "Down arrow should change selection in providers dialog"
    );
}

#[test]
fn login_flow_save_requires_validation() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    state.update(crate::Event::ProvidersDialog);
    state.update(crate::Event::ProvidersAdd);
    state.update(crate::Event::SelectProvider {
        provider: "minimax".into(),
    });
    state.update(crate::Event::SubmitKey {
        provider: "minimax".into(),
        key: "sk-test".into(),
    });
    state.update(crate::Event::Save);

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

    state.update(crate::Event::ProvidersDialog);
    state.update(crate::Event::ProvidersAdd);
    state.update(crate::Event::SelectProvider {
        provider: "minimax".into(),
    });
    state.update(crate::Event::SubmitKey {
        provider: "minimax".into(),
        key: "sk-test".into(),
    });
    validate_provider(&mut state, "minimax", "sk-test");
    state.update(crate::Event::Save);

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
    assert!(state.has_models(), "state should report a connected model");
    assert!(
        state.snapshot().has_models,
        "snapshot should expose has_models after save"
    );
}

#[test]
fn login_flow_save_saves_config() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    state.update(crate::Event::ProvidersDialog);
    state.update(crate::Event::ProvidersAdd);
    state.update(crate::Event::SelectProvider {
        provider: "minimax".into(),
    });
    state.update(crate::Event::SubmitKey {
        provider: "minimax".into(),
        key: "sk-test".into(),
    });
    validate_provider(&mut state, "minimax", "sk-test");
    state.update(crate::Event::Save);

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

    state.update(crate::Event::ProvidersDialog);
    state.update(crate::Event::ProvidersAdd);
    state.update(crate::Event::SelectProvider {
        provider: "minimax".into(),
    });
    state.update(crate::Event::SubmitKey {
        provider: "minimax".into(),
        key: "sk-test".into(),
    });
    validate_provider(&mut state, "minimax", "sk-test");
    state.update(crate::Event::Save);

    state.update(crate::Event::ProvidersSelectModel {
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

    state.update(crate::Event::ProvidersDialog);
    state.update(crate::Event::ProvidersAdd);
    state.update(crate::Event::SelectProvider {
        provider: "openai".into(),
    });
    state.update(crate::Event::SubmitKey {
        provider: "openai".into(),
        key: "sk-test".into(),
    });

    let defaults = default_models_for_provider("openai");
    validate_provider(&mut state, "openai", "sk-test");
    state.update(crate::Event::Save);

    if defaults.len() >= 2 {
        state.update(crate::Event::ProvidersSelectModel {
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
fn login_key_input_reads_typed_key() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    state.update(crate::Event::Start);
    state.update(crate::Event::SelectProvider {
        provider: "minimax".into(),
    });
    for c in "sk-test".chars() {
        state.update(crate::Event::Input(c));
    }
    state.update(crate::Event::Submit);

    let flow = state.login_flow.as_ref().unwrap();
    assert_eq!(flow.step, crate::login_flow::LoginStep::Validating);
    assert_eq!(flow.key, "sk-test");

    state.update(crate::Event::ModelsFetched {
        provider: "minimax".into(),
        key: "sk-test".into(),
        models: vec!["MiniMax-M3".into()],
    });
    let flow = state.login_flow.as_ref().unwrap();
    assert_eq!(flow.step, crate::login_flow::LoginStep::ModelSelect);
}

#[test]
fn login_key_input_submit_button_submits_typed_key() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    state.update(crate::Event::Start);
    state.update(crate::Event::SelectProvider {
        provider: "minimax".into(),
    });
    for c in "sk-test".chars() {
        state.update(crate::Event::Input(c));
    }
    // Move focus from the API Key field down to the Submit button.
    state.update(crate::Event::HistoryNext);
    state.update(crate::Event::Submit);

    let flow = state.login_flow.as_ref().unwrap();
    assert_eq!(
        flow.step,
        crate::login_flow::LoginStep::Validating,
        "pressing Enter on the Submit button should submit the typed key"
    );
    assert_eq!(flow.key, "sk-test");
}

#[test]
fn login_key_input_rejects_empty_key() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    state.update(crate::Event::Start);
    state.update(crate::Event::SelectProvider {
        provider: "minimax".into(),
    });
    state.update(crate::Event::Submit);

    let flow = state.login_flow.as_ref().unwrap();
    assert_eq!(
        flow.step,
        crate::login_flow::LoginStep::KeyInput,
        "empty key should keep the key input panel open"
    );
}

#[test]
fn login_flow_save_blocked_after_validation_failure() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    state.update(crate::Event::Start);
    state.update(crate::Event::SelectProvider {
        provider: "minimax".into(),
    });
    state.update(crate::Event::SubmitKey {
        provider: "minimax".into(),
        key: "sk-test".into(),
    });
    state.update(crate::Event::ValidationFailed {
        provider: "minimax".into(),
        key: "sk-test".into(),
        error: "bad key".into(),
    });
    state.update(crate::Event::Save);

    assert!(
        state.login_flow.is_some(),
        "save should be blocked after validation failure"
    );
    assert!(list_configured_providers().is_empty());
}

#[tokio::test]
async fn login_flow_save_does_not_block_async_runtime() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    state.update(crate::Event::ProvidersDialog);
    state.update(crate::Event::ProvidersAdd);
    state.update(crate::Event::SelectProvider {
        provider: "minimax".into(),
    });
    state.update(crate::Event::SubmitKey {
        provider: "minimax".into(),
        key: "sk-test".into(),
    });
    validate_provider(&mut state, "minimax", "sk-test");

    let start = std::time::Instant::now();
    state.update(crate::Event::Save);
    let elapsed = start.elapsed();
    assert!(
        elapsed < std::time::Duration::from_millis(100),
        "save blocked the runtime for {elapsed:?}"
    );

    for _ in 0..50 {
        if list_configured_providers().iter().any(|(n, _, _)| n == "minimax") {
            break;
        }
        tokio::task::yield_now().await;
    }
    assert!(
        list_configured_providers().iter().any(|(n, _, _)| n == "minimax"),
        "provider should be saved in the background"
    );
}

#[test]
fn login_flow_panel_changes_mark_dirty() {
    clean_config();
    let mut state = AppState::default();
    state.view.dirty = false;

    state.update(crate::Event::ProvidersDialog);
    state.update(crate::Event::ProvidersAdd);
    state.update(crate::Event::SelectProvider {
        provider: "minimax".into(),
    });

    assert!(
        state.view.dirty,
        "pushing a new login panel should mark the view dirty"
    );
}

#[test]
fn login_flow_save_updates_config_cache_for_immediate_model_switch() {
    // Regression test: after saving a provider, model switching should work
    // immediately without waiting for ConfigActor to publish ConfigLoaded.
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    state.update(crate::Event::ProvidersDialog);
    state.update(crate::Event::ProvidersAdd);
    state.update(crate::Event::SelectProvider {
        provider: "minimax".into(),
    });
    state.update(crate::Event::SubmitKey {
        provider: "minimax".into(),
        key: "sk-test".into(),
    });
    validate_provider(&mut state, "minimax", "sk-test");
    state.update(crate::Event::Save);

    // Verify ConfigState contains the provider immediately
    let configured = state.configured_providers();
    assert!(
        configured.iter().any(|(p, _, _)| p == "minimax"),
        "ConfigState should contain minimax provider immediately after save"
    );

    // Try to switch to a different model using the /model command
    let result = crate::commands::dsl::handlers::model::handle_model(&mut state, "MiniMax-M3");
    assert!(
        matches!(result, crate::commands::CommandResult::Message(ref msg) if msg.contains("Switched")),
        "model switch should succeed immediately after save, got {:?}",
        result
    );
}
