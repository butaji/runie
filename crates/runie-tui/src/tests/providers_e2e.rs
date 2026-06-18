//! End-to-end provider management tests (Layer 2 + Layer 3).
//!
//! Drives the `/providers` dialog through core events and verifies state
//! transitions and rendered UI for disconnect, fallback, and add flows.

use ratatui::{backend::TestBackend, Terminal};
use runie_core::event::{DialogEvent, LoginFlowEvent};
use runie_core::{AppState, Event};

use crate::tests::{configure_test_providers, view};

fn clean_config() {
    let dir = std::env::temp_dir().join(format!(
        "runie_providers_e2e_{:?}",
        std::thread::current().id()
    ));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("config.toml");
    let _ = std::fs::remove_file(&path);
    runie_core::login_config::set_test_config_path(path);
}

fn render_content(state: &mut AppState) -> String {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|f| view(f, state)).expect("draw");
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|c| c.symbol())
        .collect()
}

fn configured_provider_names() -> Vec<String> {
    runie_core::login_config::list_configured_providers()
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
    state.config.current_provider = "openai".into();
    state.config.current_model = "gpt-4o".into();

    state.update(DialogEvent::ProvidersDialog);
    state.update(DialogEvent::ProvidersDisconnect {
        provider: "openai".into(),
    });

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
fn add_provider_via_providers_dialog_keeps_active_model_unchanged() {
    clean_config();
    configure_test_providers(&[("openai".into(), vec!["gpt-4o".into()])]);

    let mut state = AppState::default();
    state.config.current_provider = "openai".into();
    state.config.current_model = "gpt-4o".into();
    state.set_login_validation_hook(std::sync::Arc::new(|_provider: &str, _key: &str| {}));

    state.update(DialogEvent::ProvidersDialog);
    state.update(DialogEvent::ProvidersAdd);
    state.update(Event::from(LoginFlowEvent::SelectProvider {
        provider: "minimax".into(),
    }));
    state.update(Event::from(LoginFlowEvent::SubmitKey {
        provider: "minimax".into(),
        key: "sk-test".into(),
    }));
    state.update(Event::from(LoginFlowEvent::ModelsFetched {
        provider: "minimax".into(),
        key: "sk-test".into(),
        models: vec!["MiniMax-M3".into()],
    }));
    state.update(Event::from(LoginFlowEvent::Save));

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
    state.update(DialogEvent::ProvidersDialog);
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
