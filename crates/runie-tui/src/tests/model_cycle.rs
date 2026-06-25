//! Layer 2 + Layer 3 tests for Ctrl+M model cycling.

use super::*;
use runie_core::Event;
use runie_core::model::{AppState, ScopedModel};

fn scoped_model(provider: &str, name: &str) -> ScopedModel {
    ScopedModel {
        provider: provider.into(),
        name: name.into(),
        enabled: true,
    }
}

fn footer_content(state: &mut AppState) -> String {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|f| view(f, state)).expect("draw");
    let buf = terminal.backend().buffer();
    buf.content.iter().map(|c| c.symbol()).collect()
}

#[test]
fn ctrl_m_cycles_to_next_scoped_model() {
    let mut state = AppState::default();
    state.config.scoped_models = vec![
        scoped_model("openai", "gpt-4o"),
        scoped_model("anthropic", "claude-3-sonnet"),
    ];
    state.config.scoped_index = 0;
    state.config.current_provider = "openai".into();
    state.config.current_model = "gpt-4o".into();

    state.update(Event::CycleModelNext);

    assert_eq!(state.config.current_provider, "anthropic");
    assert_eq!(state.config.current_model, "claude-3-sonnet");
}

#[test]
fn ctrl_m_wraps_to_first_model() {
    let mut state = AppState::default();
    state.config.scoped_models = vec![
        scoped_model("openai", "gpt-4o"),
        scoped_model("anthropic", "claude-3-sonnet"),
    ];
    state.config.scoped_index = 1;
    state.config.current_provider = "anthropic".into();
    state.config.current_model = "claude-3-sonnet".into();

    state.update(Event::CycleModelNext);

    assert_eq!(state.config.current_provider, "openai");
    assert_eq!(state.config.current_model, "gpt-4o");
}

#[test]
fn ctrl_m_updates_footer_model_name() {
    let mut state = AppState::default();
    state.config.scoped_models = vec![
        scoped_model("openai", "gpt-4o"),
        scoped_model("anthropic", "claude-3-sonnet"),
    ];
    state.config.scoped_index = 0;
    state.config.current_provider = "openai".into();
    state.config.current_model = "gpt-4o".into();
    state.cwd_name = "testdir".into();

    let before = footer_content(&mut state);
    assert!(
        before.contains("openai/gpt-4o"),
        "footer should show initial model: {before}"
    );

    state.update(Event::CycleModelNext);
    let after = footer_content(&mut state);
    assert!(
        after.contains("anthropic/claude-3-sonnet"),
        "footer should show cycled model: {after}"
    );
}

#[test]
fn ctrl_m_wraps_footer_back_to_first() {
    let mut state = AppState::default();
    state.config.scoped_models = vec![
        scoped_model("openai", "gpt-4o"),
        scoped_model("anthropic", "claude-3-sonnet"),
    ];
    state.config.scoped_index = 1;
    state.config.current_provider = "anthropic".into();
    state.config.current_model = "claude-3-sonnet".into();
    state.cwd_name = "testdir".into();

    state.update(Event::CycleModelNext);
    let content = footer_content(&mut state);
    assert!(
        content.contains("openai/gpt-4o"),
        "footer should wrap back to first model: {content}"
    );
}
