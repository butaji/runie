//! End-to-end tests for onboarding input interactions (Layer 2 + Layer 3).
//!
//! These tests drive the login/provider-add flow through core input events and
//! verify that typing, backspace, paste, toggles, and submit/cancel behave as
//! expected.

use runie_core::event::{InputEvent, LoginFlowEvent};
use runie_core::{AppState, Event};

fn clean_config() {
    let dir = std::env::temp_dir().join(format!(
        "runie_onboarding_input_{:?}",
        std::thread::current().id()
    ));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("config.toml");
    let _ = std::fs::remove_file(&path);
    runie_core::login_config::set_test_config_path(path);
}

fn current_panel(state: &AppState) -> Option<&runie_core::dialog::Panel> {
    state
        .open_dialog
        .as_ref()
        .and_then(|d| d.panel_stack())
        .and_then(|s| s.current())
}

fn start_provider_select(state: &mut AppState, provider: &str) {
    state.update(Event::from(LoginFlowEvent::Start));
    state.update(Event::from(LoginFlowEvent::SelectProvider {
        provider: provider.into(),
    }));
}

fn reach_model_select(state: &mut AppState, models: &[String]) {
    start_provider_select(state, "minimax");
    state.update(Event::from(LoginFlowEvent::SubmitKey {
        provider: "minimax".into(),
        key: "sk-test".into(),
    }));
    state.update(Event::from(LoginFlowEvent::ModelsFetched {
        provider: "minimax".into(),
        key: "sk-test".into(),
        models: models.to_vec(),
    }));
}

#[test]
fn type_api_key_appears_in_field() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();
    state.set_login_validation_hook(std::sync::Arc::new(|_provider: &str, _key: &str| {}));

    start_provider_select(&mut state, "minimax");
    for c in "sk-test".chars() {
        state.update(Event::from(InputEvent::Input(c)));
    }

    let panel = current_panel(&state).expect("key input panel should be open");
    assert_eq!(panel.id, "login-key");
    assert_eq!(
        panel.form_values.get("key"),
        Some(&"sk-test".to_string()),
        "typed API key should appear in form_values"
    );
}

#[test]
fn backspace_removes_key_character() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();
    state.set_login_validation_hook(std::sync::Arc::new(|_provider: &str, _key: &str| {}));

    start_provider_select(&mut state, "minimax");
    for c in "sk-test".chars() {
        state.update(Event::from(InputEvent::Input(c)));
    }
    state.update(Event::from(InputEvent::Backspace));

    let panel = current_panel(&state).expect("key input panel should be open");
    assert_eq!(panel.id, "login-key");
    assert_eq!(
        panel.form_values.get("key"),
        Some(&"sk-tes".to_string()),
        "backspace should remove the last character"
    );
}

#[test]
fn paste_fills_api_key_field() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();
    state.set_login_validation_hook(std::sync::Arc::new(|_provider: &str, _key: &str| {}));

    start_provider_select(&mut state, "minimax");
    state.update(Event::Paste("sk-pasted".into()));

    let panel = current_panel(&state).expect("key input panel should be open");
    assert_eq!(panel.id, "login-key");
    assert_eq!(
        panel.form_values.get("key"),
        Some(&"sk-pasted".to_string()),
        "paste should fill the API key field"
    );
}

#[test]
fn space_toggles_model_checkbox_in_login_flow() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();
    state.set_login_validation_hook(std::sync::Arc::new(|_provider: &str, _key: &str| {}));

    reach_model_select(&mut state, &["MiniMax-M3".into()]);

    state.update(Event::from(InputEvent::Input(' ')));

    let flow = state
        .login_flow
        .as_ref()
        .expect("login flow should be active");
    assert!(
        !flow.selected_models.contains("MiniMax-M3"),
        "space should deselect the model"
    );

    let panel = current_panel(&state).expect("model selector should still be open");
    assert_eq!(panel.id, "login-models");
}

#[test]
fn enter_on_save_action_saves() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();
    state.set_login_validation_hook(std::sync::Arc::new(|_provider: &str, _key: &str| {}));

    reach_model_select(&mut state, &["MiniMax-M3".into()]);
    // Move selection from the model toggle down to the _Save action.
    state.update(Event::from(InputEvent::HistoryNext));
    state.update(Event::from(InputEvent::Submit));

    assert!(
        state.open_dialog.is_none(),
        "dialog should close after save"
    );
    assert!(
        state.has_models(),
        "provider/model should be active after save"
    );
    assert_eq!(state.config.current_provider, "minimax");
    assert_eq!(state.config.current_model, "MiniMax-M3");
}

#[test]
fn enter_on_unchecked_model_selects_and_saves() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();
    state.set_login_validation_hook(std::sync::Arc::new(|_provider: &str, _key: &str| {}));

    reach_model_select(&mut state, &["MiniMax-M3".into(), "MiniMax-M2.7".into()]);
    // Uncheck the first model.
    state.update(Event::from(InputEvent::Input(' ')));
    {
        let flow = state.login_flow.as_ref().unwrap();
        assert!(!flow.selected_models.contains("MiniMax-M3"));
        assert!(flow.selected_models.contains("MiniMax-M2.7"));
    }

    // Press Enter on the unchecked first model to reselect it and save.
    state.update(Event::from(InputEvent::Submit));

    assert!(
        state.open_dialog.is_none(),
        "dialog should close after save"
    );
    assert!(
        state.has_models(),
        "provider/model should be active after save"
    );
    assert_eq!(state.config.current_provider, "minimax");
    assert_eq!(state.config.current_model, "MiniMax-M3");
}

#[test]
fn cancel_action_returns_to_previous_panel() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();
    state.set_login_validation_hook(std::sync::Arc::new(|_provider: &str, _key: &str| {}));

    reach_model_select(&mut state, &["MiniMax-M3".into()]);
    // Move selection from the model toggle down to _Save, then to _Cancel.
    state.update(Event::from(InputEvent::HistoryNext));
    state.update(Event::from(InputEvent::HistoryNext));
    state.update(Event::from(InputEvent::Submit));

    let panel = current_panel(&state).expect("dialog should still be open after cancel");
    assert_eq!(
        panel.id, "login-key",
        "cancel should return to the API key input panel"
    );
}

#[test]
fn paste_multiline_api_key_preserves_full_text() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();
    state.set_login_validation_hook(std::sync::Arc::new(|_provider: &str, _key: &str| {}));

    start_provider_select(&mut state, "minimax");
    state.update(Event::Paste("sk-line1\nline2\nline3".into()));

    let panel = current_panel(&state).expect("key input panel should be open");
    assert_eq!(panel.id, "login-key");
    assert_eq!(
        panel.form_values.get("key"),
        Some(&"sk-line1\nline2\nline3".to_string()),
        "multi-line paste should keep newlines in the API key field"
    );

    // No accidental submit: the dialog should still be on the key panel.
    assert_eq!(
        state
            .open_dialog
            .as_ref()
            .and_then(|d| d.panel_stack())
            .and_then(|s| s.current())
            .map(|p| p.id.as_str()),
        Some("login-key")
    );
}
