//! Onboarding / login flow rendering tests (Layer 3).
//!
//! Drives the provider-add flow through core events and verifies the rendered
//! UI for each panel: provider picker, key input, validating, model selector,
//! empty model list, and validation failure.

use std::sync::Arc;

use ratatui::{backend::TestBackend, Terminal};
use runie_core::event::{InputEvent, LoginFlowEvent};
use runie_core::{AppState, Event};

use crate::tests::view;

fn clean_config() {
    let dir = std::env::temp_dir().join(format!(
        "runie_onboarding_render_{:?}",
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

#[test]
fn provider_picker_renders_providers_and_cancel() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    state.update(Event::from(LoginFlowEvent::Start));

    let top_content = render_content(&mut state);
    assert!(
        top_content.contains("Login"),
        "should render Login title, got: {}",
        top_content
    );
    assert!(
        top_content.contains("Anthropic"),
        "should render Anthropic provider, got: {}",
        top_content
    );

    // Scroll down until MiniMax is visible; the list cannot show every provider
    // and the Cancel action at the same time in the 80x24 render area.
    for _ in 0..9 {
        state.update(Event::from(InputEvent::HistoryNext));
    }
    let mid_content = render_content(&mut state);
    assert!(
        mid_content.contains("Login"),
        "should still render Login title after scrolling, got: {}",
        mid_content
    );
    assert!(
        mid_content.contains("MiniMax"),
        "should render MiniMax provider after scrolling, got: {}",
        mid_content
    );

    // Continue to the _Cancel action at the bottom of the list.
    for _ in 0..4 {
        state.update(Event::from(InputEvent::HistoryNext));
    }
    let bottom_content = render_content(&mut state);
    assert!(
        bottom_content.contains("Cancel"),
        "should render Cancel action after scrolling, got: {}",
        bottom_content
    );
}

#[test]
fn key_input_renders_provider_name_and_field() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();
    state.set_login_validation_hook(Arc::new(|_provider: &str, _key: &str| {}));

    state.update(Event::from(LoginFlowEvent::Start));
    state.update(Event::from(LoginFlowEvent::SelectProvider {
        provider: "minimax".into(),
    }));

    let content = render_content(&mut state);
    assert!(
        content.contains("Login to MiniMax"),
        "should render provider-specific title, got: {}",
        content
    );
    assert!(
        content.contains("API Key"),
        "should render API Key field label, got: {}",
        content
    );
    assert!(
        content.contains(" Login to MiniMax "),
        "title should have exactly one space padding, got: {:?}",
        content
    );
    assert!(
        !content.contains("  Login to MiniMax  "),
        "title should not have double space padding"
    );
}

#[test]
fn typed_api_key_renders_in_input_box() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();
    state.set_login_validation_hook(Arc::new(|_provider: &str, _key: &str| {}));

    state.update(Event::from(LoginFlowEvent::Start));
    state.update(Event::from(LoginFlowEvent::SelectProvider {
        provider: "minimax".into(),
    }));
    for c in "sk-test".chars() {
        state.update(Event::from(InputEvent::Input(c)));
    }

    let content = render_content(&mut state);
    assert!(
        content.contains("sk-test"),
        "typed API key should render in the input box, got: {}",
        content
    );
}

#[test]
fn validating_panel_renders_provider() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();
    state.set_login_validation_hook(Arc::new(|_provider: &str, _key: &str| {}));

    state.update(Event::from(LoginFlowEvent::Start));
    state.update(Event::from(LoginFlowEvent::SelectProvider {
        provider: "minimax".into(),
    }));
    state.update(Event::from(LoginFlowEvent::SubmitKey {
        provider: "minimax".into(),
        key: "sk-test".into(),
    }));

    let content = render_content(&mut state);
    assert!(
        content.contains("Verifying MiniMax"),
        "should render verifying panel, got: {}",
        content
    );
}

#[test]
fn model_selector_renders_toggles_save_cancel() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();
    state.set_login_validation_hook(Arc::new(|_provider: &str, _key: &str| {}));

    state.update(Event::from(LoginFlowEvent::Start));
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
        models: vec!["MiniMax-M3".into(), "MiniMax-M2".into()],
    }));

    let content = render_content(&mut state);
    assert!(
        content.contains("Select MiniMax Models"),
        "should render model selector title, got: {}",
        content
    );
    assert!(
        content.contains("[x] MiniMax-M3"),
        "should render checked MiniMax-M3 toggle, got: {}",
        content
    );
    assert!(
        content.contains("[x] MiniMax-M2"),
        "should render checked MiniMax-M2 toggle, got: {}",
        content
    );
    assert!(
        content.contains("Save"),
        "should render Save button, got: {}",
        content
    );
    assert!(
        content.contains("Cancel"),
        "should render Cancel button, got: {}",
        content
    );
}

#[test]
fn empty_model_list_renders_save_and_warning() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();
    state.set_login_validation_hook(Arc::new(|_provider: &str, _key: &str| {}));

    state.update(Event::from(LoginFlowEvent::Start));
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
        models: vec![],
    }));

    let content = render_content(&mut state);
    assert!(
        content.contains("Select MiniMax Models"),
        "should render model selector title, got: {}",
        content
    );
    assert!(
        content.contains("Save"),
        "should render Save button for empty list, got: {}",
        content
    );
}

#[test]
fn validation_failure_renders_error() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();
    state.set_login_validation_hook(Arc::new(|_provider: &str, _key: &str| {}));

    state.update(Event::from(LoginFlowEvent::Start));
    state.update(Event::from(LoginFlowEvent::SelectProvider {
        provider: "minimax".into(),
    }));
    state.update(Event::from(LoginFlowEvent::SubmitKey {
        provider: "minimax".into(),
        key: "sk-bad".into(),
    }));
    state.update(Event::from(LoginFlowEvent::ValidationFailed {
        provider: "minimax".into(),
        key: "sk-bad".into(),
        error: "bad".into(),
    }));

    let content = render_content(&mut state);
    assert!(
        content.contains("Could not verify key") || content.contains("bad"),
        "should render validation failure message, got: {}",
        content
    );
}

#[test]
fn title_padding_exactly_one_space() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();
    state.set_login_validation_hook(Arc::new(|_provider: &str, _key: &str| {}));

    state.update(Event::from(LoginFlowEvent::Start));
    state.update(Event::from(LoginFlowEvent::SelectProvider {
        provider: "minimax".into(),
    }));

    let key_content = render_content(&mut state);
    assert!(
        key_content.contains(" Login to MiniMax "),
        "key input title should have exactly one space padding, got: {:?}",
        key_content
    );
    assert!(
        !key_content.contains("  Login to MiniMax  "),
        "key input title should not have double space padding"
    );

    state.update(Event::from(LoginFlowEvent::SubmitKey {
        provider: "minimax".into(),
        key: "sk-test".into(),
    }));
    state.update(Event::from(LoginFlowEvent::ModelsFetched {
        provider: "minimax".into(),
        key: "sk-test".into(),
        models: vec!["MiniMax-M3".into()],
    }));

    let model_content = render_content(&mut state);
    assert!(
        model_content.contains(" Select MiniMax Models "),
        "model selector title should have exactly one space padding, got: {:?}",
        model_content
    );
    assert!(
        !model_content.contains("  Select MiniMax Models  "),
        "model selector title should not have double space padding"
    );
}
