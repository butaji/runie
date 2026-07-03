#![allow(clippy::useless_conversion)]
//! End-to-end tests for login form editing (Layer 2 + Layer 3).
//!
//! Covers typing, pasting, and cursor navigation inside the API key form field.

use super::*;
use runie_core::Event;

fn clean_config() {
    let path = runie_core::provider::config::generate_test_config_path("runie_login_form");
    let _ = std::fs::remove_file(&path);
    runie_core::provider::config::set_test_config_path(path);
}

#[test]
fn e2e_login_flow_paste_fills_api_key_field() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    state.update(Event::from(Event::Start));
    state.update(Event::from(Event::SelectProvider {
        provider: "minimax".into(),
    }));
    state.update(Event::Paste("sk-pasted-from-clipboard".into()));

    let stack = state
        .open_dialog
        .as_ref()
        .and_then(|d| d.panel_stack())
        .expect("key input panel should be open");
    let panel = stack.current().expect("current panel");
    assert_eq!(
        panel.form_values.get("key"),
        Some(&"sk-pasted-from-clipboard".to_string())
    );
}

#[test]
fn e2e_login_flow_typing_renders_api_key_field() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    state.update(Event::from(Event::Start));
    state.update(Event::from(Event::SelectProvider {
        provider: "minimax".into(),
    }));
    for c in "sk-typed".chars() {
        state.update(Event::from(Event::Input(c)));
    }

    let content = render_content(&mut state);
    assert!(
        content.contains("sk-typed"),
        "Typed API key must be visible in the login form. Got: {}",
        content
    );
}

#[test]
fn e2e_providers_add_flow_typing_renders_api_key_field() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    state.update(Event::from(Event::ProvidersDialog));
    state.update(Event::from(Event::ProvidersAdd));
    state.update(Event::from(Event::SelectProvider {
        provider: "minimax".into(),
    }));
    for c in "sk-from-providers".chars() {
        state.update(Event::from(Event::Input(c)));
    }

    let content = render_content(&mut state);
    assert!(
        content.contains("sk-from-providers"),
        "Typed API key must be visible after opening login from providers dialog. Got: {}",
        content
    );
}

#[test]
fn e2e_login_flow_cursor_left_allows_inline_editing() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    state.update(Event::from(Event::Start));
    state.update(Event::from(Event::SelectProvider {
        provider: "minimax".into(),
    }));
    for c in "sk-tyed".chars() {
        state.update(Event::from(Event::Input(c)));
    }
    state.update(Event::from(Event::CursorLeft));
    state.update(Event::from(Event::CursorLeft));
    state.update(Event::from(Event::Input('p')));

    let content = render_content(&mut state).replace('▏', "");
    assert!(
        content.contains("sk-typed"),
        "Moving cursor left and inserting must fix typo. Got: {}",
        content
    );
}
