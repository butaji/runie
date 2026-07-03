//! Model cycling tests (Layer 1 + Layer 2)

use crate::model::{AppState, ScopedModel};

fn sm(provider: &str, name: &str, enabled: bool) -> ScopedModel {
    ScopedModel {
        provider: provider.into(),
        name: name.into(),
        enabled,
    }
}

#[test]
fn cycle_next_increments() {
    let mut state = AppState::default();
    state.config.scoped_models = vec![
        sm("mock", "echo", true),
        sm("openai", "gpt-4o", true),
        sm("anthropic", "claude-3-sonnet", true),
    ];
    state.config.scoped_index = 0;

    state.update(crate::Event::CycleModelNext);
    assert_eq!(state.config.scoped_index, 1);
}

#[test]
fn cycle_prev_decrements() {
    let mut state = AppState::default();
    state.config.scoped_models = vec![
        sm("mock", "echo", true),
        sm("openai", "gpt-4o", true),
        sm("anthropic", "claude-3-sonnet", true),
    ];
    state.config.scoped_index = 1;

    state.update(crate::Event::CycleModelPrev);
    assert_eq!(state.config.scoped_index, 0);
}

#[test]
fn cycle_wraps_forward() {
    let mut state = AppState::default();
    state.config.scoped_models = vec![sm("mock", "echo", true), sm("openai", "gpt-4o", true)];
    state.config.scoped_index = 1;

    state.update(crate::Event::CycleModelNext);
    assert_eq!(state.config.scoped_index, 0);
}

#[test]
fn cycle_wraps_backward() {
    let mut state = AppState::default();
    state.config.scoped_models = vec![sm("mock", "echo", true), sm("openai", "gpt-4o", true)];
    state.config.scoped_index = 0;

    state.update(crate::Event::CycleModelPrev);
    assert_eq!(state.config.scoped_index, 1);
}

#[test]
fn cycle_empty_noop() {
    let mut state = AppState::default();
    state.config.current_provider = "mock".into();
    state.config.current_model = "echo".into();
    state.config.scoped_models = vec![];
    state.config.scoped_index = 0;

    state.update(crate::Event::CycleModelNext);
    assert_eq!(state.config.scoped_index, 0);
    assert_eq!(state.config.current_provider, "mock");
    assert_eq!(state.config.current_model, "echo");
}

#[test]
fn cycle_emits_switch_model() {
    let mut state = AppState::default();
    state.config.scoped_models = vec![sm("mock", "echo", true), sm("openai", "gpt-4o", true)];
    state.config.scoped_index = 0;

    state.update(crate::Event::CycleModelNext);

    assert_eq!(state.config.current_provider, "openai");
    assert_eq!(state.config.current_model, "gpt-4o");

    assert_eq!(
        state.transient_message,
        Some("Switched to openai/gpt-4o".into())
    );
    assert_eq!(
        state.transient_level,
        Some(crate::event::TransientLevel::Success)
    );
}

#[test]
fn cycle_skips_disabled_models() {
    let mut state = AppState::default();
    state.config.scoped_models = vec![
        sm("mock", "echo", true),
        sm("openai", "gpt-4o", false),
        sm("anthropic", "claude-3", true),
    ];
    state.config.scoped_index = 0;

    state.update(crate::Event::CycleModelNext);

    // Should skip disabled gpt-4o and land on claude-3
    assert_eq!(state.config.scoped_index, 2);
    assert_eq!(state.config.current_provider, "anthropic");
    assert_eq!(state.config.current_model, "claude-3");
}

#[test]
fn cycle_all_disabled_noop() {
    let mut state = AppState::default();
    state.config.current_provider = "mock".into();
    state.config.current_model = "echo".into();
    state.config.scoped_models = vec![sm("mock", "echo", false), sm("openai", "gpt-4o", false)];
    state.config.scoped_index = 0;

    state.update(crate::Event::CycleModelNext);
    assert_eq!(state.config.scoped_index, 0);
    assert_eq!(state.config.current_provider, "mock");
    assert_eq!(state.config.current_model, "echo");
}
