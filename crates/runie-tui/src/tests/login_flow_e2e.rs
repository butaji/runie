#![allow(clippy::useless_conversion)]
//! End-to-end login flow tests (Layer 2 + Layer 3).
//!
//! Drives the provider-add flow through core events and verifies both the
//! state transitions and the rendered UI, including the async validation hook.

use super::*;
use runie_core::login_flow::LoginStep;
use runie_core::Event;

fn clean_config() {
    let path = runie_core::provider::config::generate_test_config_path("runie_login_e2e");
    let _ = std::fs::remove_file(&path);
    runie_core::provider::config::set_test_config_path(path);
}

#[test]
fn e2e_login_flow_shows_verifying_panel() {
    clean_config();

    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    state.update(Event::from(Event::Start));
    state.update(Event::from(Event::SelectProvider {
        provider: "minimax".into(),
    }));
    state.update(Event::from(Event::SubmitKey {
        provider: "minimax".into(),
        key: "sk-test".into(),
    }));

    assert_eq!(
        state.login_flow.as_ref().unwrap().step,
        LoginStep::Validating
    );

    let content = render_content(&mut state);
    assert!(
        content.contains("Verifying MiniMax"),
        "should render verifying panel, got: {}",
        content
    );
}

#[test]
fn e2e_login_flow_reaches_model_selector() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    state.update(Event::from(Event::Start));
    state.update(Event::from(Event::SelectProvider {
        provider: "minimax".into(),
    }));
    state.update(Event::from(Event::SubmitKey {
        provider: "minimax".into(),
        key: "sk-test".into(),
    }));
    state.update(Event::from(Event::ModelsFetched {
        provider: "minimax".into(),
        key: "sk-test".into(),
        models: vec!["MiniMax-M3".into(), "MiniMax-M2.7".into()],
    }));

    assert_eq!(
        state.login_flow.as_ref().unwrap().step,
        LoginStep::ModelSelect
    );

    let content = render_content(&mut state);
    assert!(
        content.contains("Select MiniMax Models"),
        "should render model selector, got: {}",
        content
    );
    assert!(content.contains("MiniMax-M3"));
    assert!(content.contains("MiniMax-M2.7"));
}

#[test]
fn e2e_login_flow_save_activates_first_model() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    state.update(Event::from(Event::Start));
    state.update(Event::from(Event::SelectProvider {
        provider: "minimax".into(),
    }));
    state.update(Event::from(Event::SubmitKey {
        provider: "minimax".into(),
        key: "sk-test".into(),
    }));
    state.update(Event::from(Event::ModelsFetched {
        provider: "minimax".into(),
        key: "sk-test".into(),
        models: vec!["MiniMax-M3".into()],
    }));
    state.update(Event::from(Event::Save));

    assert!(state.login_flow.is_none(), "flow should close after save");
    assert_eq!(state.config.current_provider, "minimax");
    assert_eq!(state.config.current_model, "MiniMax-M3");
}

#[test]
fn e2e_providers_select_model_renders_input_box() {
    clean_config();
    runie_core::provider::config::save_provider_config(
        "minimax",
        "https://api.minimaxi.chat/v1",
        "sk-test",
        &["MiniMax-M3".into()],
    )
    .unwrap();

    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    state.update(Event::ProvidersDialog);
    state.update(Event::ProvidersSelectModel {
        provider: "minimax".into(),
        model: "MiniMax-M3".into(),
    });

    assert!(
        state.has_models(),
        "state should report has_models after selecting a model"
    );
    assert!(
        state.open_dialog.is_none(),
        "dialog should close after selecting a model"
    );
    let content = render_content(&mut state);
    assert!(
        content.contains(" minimax/MiniMax-M3 "),
        "input box title should appear after selecting provider/model, got: {}",
        content
    );
}

#[test]
fn e2e_login_flow_title_has_exactly_one_space_padding() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    state.update(Event::from(Event::Start));
    state.update(Event::from(Event::SelectProvider {
        provider: "minimax".into(),
    }));

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();

    assert!(
        content.contains(" Login to MiniMax "),
        "title should have exactly one space before and after, got relevant content: {:?}",
        content
    );
    assert!(
        !content.contains("  Login to MiniMax  "),
        "title should not have more than one space before/after"
    );
}

#[test]
fn e2e_providers_add_flow_save_renders_input_box() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

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

    assert!(
        state.has_models(),
        "state should report has_models after save"
    );
    assert!(
        state.open_dialog.is_none(),
        "dialog should close after save"
    );
    let content = render_content(&mut state);
    assert!(
        content.contains(" minimax/MiniMax-M3 "),
        "input box title should appear after connecting provider/model, got: {}",
        content
    );
}

#[test]
fn e2e_login_flow_submit_save_button_renders_input_box() {
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
    // Move selection from the model toggle down to the _Save action.
    state.update(Event::HistoryNext);
    state.update(Event::Submit);

    assert!(
        state.has_models(),
        "state should report has_models after save"
    );
    assert!(
        state.open_dialog.is_none(),
        "dialog should close after save"
    );
    let content = render_content(&mut state);
    assert!(
        content.contains(" minimax/MiniMax-M3 "),
        "input box title should appear after activating Save, got: {}",
        content
    );
}

#[test]
fn e2e_login_flow_save_renders_input_box() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    state.update(Event::from(Event::Start));
    state.update(Event::from(Event::SelectProvider {
        provider: "minimax".into(),
    }));
    state.update(Event::from(Event::SubmitKey {
        provider: "minimax".into(),
        key: "sk-test".into(),
    }));
    state.update(Event::from(Event::ModelsFetched {
        provider: "minimax".into(),
        key: "sk-test".into(),
        models: vec!["MiniMax-M3".into()],
    }));
    state.update(Event::from(Event::Save));

    assert!(
        state.has_models(),
        "state should report has_models after save"
    );
    let content = render_content(&mut state);
    assert!(
        content.contains(" minimax/MiniMax-M3 "),
        "input box title should appear after connecting provider/model, got: {}",
        content
    );
}

#[test]
fn e2e_login_flow_submit_on_model_toggle_saves_and_connects() {
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
    // Press Enter while a model toggle is selected.
    state.update(Event::Submit);

    assert!(
        state.open_dialog.is_none(),
        "dialog should close and connect after Enter on a model toggle"
    );
    assert!(
        state.has_models(),
        "provider/model should be active after Enter save"
    );
    let content = render_content(&mut state);
    assert!(
        content.contains(" minimax/MiniMax-M3 "),
        "input box title should appear after Enter on model toggle, got: {}",
        content
    );
}

#[test]
fn e2e_login_flow_api_key_label_renders_fully() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    state.update(Event::from(Event::Start));
    state.update(Event::from(Event::SelectProvider {
        provider: "minimax".into(),
    }));

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();

    assert!(
        content.contains("API Key"),
        "API Key label should render fully, got: {}",
        content
    );
    // The label row must contain the full word; a clipped half-character would
    // not pass this assertion.
    assert!(
        content.contains("  1. API Key"),
        "label row should include the numeric marker and full label"
    );
}

#[test]
fn e2e_reset_preserves_input_box() {
    let mut state = AppState::default();
    connect_model(&mut state);
    assert!(
        render_content(&mut state).contains(" openai/gpt-4o "),
        "input box should render before reset"
    );

    state.update(Event::reset());

    assert!(
        state.has_models(),
        "provider/model must stay active after reset"
    );
    let content = render_content(&mut state);
    assert!(
        content.contains(" openai/gpt-4o "),
        "input box title should still render after /reset, got: {}",
        content
    );
}
