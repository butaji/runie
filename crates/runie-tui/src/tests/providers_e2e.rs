//! End-to-end provider management tests (Layer 2 + Layer 3).
//!
//! Drives the `/providers` dialog through core events and verifies state
//! transitions and rendered UI for disconnect, fallback, and add flows.

use super::*;
use runie_core::Event;

fn clean_config() {
    let path = runie_core::provider::config::generate_test_config_path("runie_providers_e2e");
    let _ = std::fs::remove_file(&path);
    runie_core::provider::config::set_test_config_path(path);
}

fn configured_provider_names() -> Vec<String> {
    runie_core::provider::config::list_configured_providers()
        .into_iter()
        .map(|(name, _, _)| name)
        .collect()
}

#[test]
fn disconnect_active_provider_switches_to_fallback() {
    clean_config();
    configure_test_providers(&[
        ("openai".into(), vec!["gpt-4o".into()]),
        ("minimax".into(), vec!["MiniMax-M3".into()]),
    ]);

    let mut state = AppState::default();
    apply_test_config_to_state(&mut state);
    state.config.current_provider = "openai".into();
    state.config.current_model = "gpt-4o".into();

    state.update(Event::ProvidersDialog);
    state.update(Event::ProvidersDisconnect { provider: "openai".into() });

    assert!(
        state.has_models(),
        "a fallback provider/model should still be active"
    );
    assert_eq!(
        state.config.current_provider, "minimax",
        "active provider should switch to fallback"
    );
    assert_eq!(
        state.config.current_model, "MiniMax-M3",
        "active model should switch to fallback"
    );

    let content = render_content(&mut state);
    assert!(
        content.contains(" minimax/MiniMax-M3 "),
        "input box title should show fallback provider/model, got: {}",
        content
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn add_provider_via_providers_dialog_keeps_active_model_unchanged() {
    clean_config();
    configure_test_providers(&[("openai".into(), vec!["gpt-4o".into()])]);

    let mut state = AppState::default();
    apply_test_config_to_state(&mut state);
    state.config.current_provider = "openai".into();
    state.config.current_model = "gpt-4o".into();

    state.update(Event::ProvidersDialog);
    state.update(Event::ProvidersAdd);
    state.update(Event::SelectProvider { provider: "minimax".into() });
    state.update(Event::SubmitKey { provider: "minimax".into(), key: "sk-test".into() });
    state.update(Event::ModelsFetched {
        provider: "minimax".into(),
        key: "sk-test".into(),
        models: vec!["MiniMax-M3".into()],
    });
    state.update(Event::Save);

    assert!(
        configured_provider_names().contains(&"openai".into()),
        "original provider should still be configured"
    );
    assert!(
        configured_provider_names().contains(&"minimax".into()),
        "new provider should be configured"
    );

    assert!(
        state.open_dialog.is_none(),
        "dialog should close after saving the new provider"
    );
    assert_eq!(
        state.config.current_provider, "openai",
        "active provider should remain unchanged"
    );
    assert_eq!(
        state.config.current_model, "gpt-4o",
        "active model should remain unchanged"
    );

    // Reopen providers dialog and verify both providers are listed.
    apply_test_config_to_state(&mut state);
    state.update(Event::ProvidersDialog);
    let content = render_content(&mut state);
    assert!(
        content.contains("openai"),
        "providers dialog should list openai, got: {}",
        content
    );
    assert!(
        content.contains("minimax"),
        "providers dialog should list minimax, got: {}",
        content
    );
}
