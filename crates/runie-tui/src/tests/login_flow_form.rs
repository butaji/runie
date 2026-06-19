//! End-to-end tests for login form editing (Layer 2 + Layer 3).
//!
//! Covers typing, pasting, and cursor navigation inside the API key form field.

use ratatui::{backend::TestBackend, Terminal};
use runie_core::event::{DialogEvent, InputEvent, LoginFlowEvent};
use runie_core::{AppState, Event};

use crate::tests::view;

fn clean_config() {
    let dir =
        std::env::temp_dir().join(format!("runie_login_form_{:?}", std::thread::current().id()));
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

#[test]
fn e2e_login_flow_paste_fills_api_key_field() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();
    state.set_login_validation_hook(std::sync::Arc::new(|_provider: &str, _key: &str| {}));

    state.update(Event::from(LoginFlowEvent::Start));
    state.update(Event::from(LoginFlowEvent::SelectProvider {
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
    state.set_login_validation_hook(std::sync::Arc::new(|_provider: &str, _key: &str| {}));

    state.update(Event::from(LoginFlowEvent::Start));
    state.update(Event::from(LoginFlowEvent::SelectProvider {
        provider: "minimax".into(),
    }));
    for c in "sk-typed".chars() {
        state.update(Event::from(InputEvent::Input(c)));
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
    state.set_login_validation_hook(std::sync::Arc::new(|_provider: &str, _key: &str| {}));

    state.update(Event::from(DialogEvent::ProvidersDialog));
    state.update(Event::from(DialogEvent::ProvidersAdd));
    state.update(Event::from(LoginFlowEvent::SelectProvider {
        provider: "minimax".into(),
    }));
    for c in "sk-from-providers".chars() {
        state.update(Event::from(InputEvent::Input(c)));
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
    state.set_login_validation_hook(std::sync::Arc::new(|_provider: &str, _key: &str| {}));

    state.update(Event::from(LoginFlowEvent::Start));
    state.update(Event::from(LoginFlowEvent::SelectProvider {
        provider: "minimax".into(),
    }));
    for c in "sk-tyed".chars() {
        state.update(Event::from(InputEvent::Input(c)));
    }
    state.update(Event::from(InputEvent::CursorLeft));
    state.update(Event::from(InputEvent::CursorLeft));
    state.update(Event::from(InputEvent::Input('p')));

    let content = render_content(&mut state).replace('▏', "");
    assert!(
        content.contains("sk-typed"),
        "Moving cursor left and inserting must fix typo. Got: {}",
        content
    );
}
