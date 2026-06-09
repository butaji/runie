//! Model cycling tests (Layer 1 + Layer 2)

use crate::event::Event;
use crate::model::{AppState, Role, ScopedModel};

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
    state.scoped_models = vec![
        sm("mock", "echo", true),
        sm("openai", "gpt-4o", true),
        sm("anthropic", "claude-3-sonnet", true),
    ];
    state.scoped_index = 0;

    state.update(Event::CycleModelNext);
    assert_eq!(state.scoped_index, 1);
}

#[test]
fn cycle_prev_decrements() {
    let mut state = AppState::default();
    state.scoped_models = vec![
        sm("mock", "echo", true),
        sm("openai", "gpt-4o", true),
        sm("anthropic", "claude-3-sonnet", true),
    ];
    state.scoped_index = 1;

    state.update(Event::CycleModelPrev);
    assert_eq!(state.scoped_index, 0);
}

#[test]
fn cycle_wraps_forward() {
    let mut state = AppState::default();
    state.scoped_models = vec![
        sm("mock", "echo", true),
        sm("openai", "gpt-4o", true),
    ];
    state.scoped_index = 1;

    state.update(Event::CycleModelNext);
    assert_eq!(state.scoped_index, 0);
}

#[test]
fn cycle_wraps_backward() {
    let mut state = AppState::default();
    state.scoped_models = vec![
        sm("mock", "echo", true),
        sm("openai", "gpt-4o", true),
    ];
    state.scoped_index = 0;

    state.update(Event::CycleModelPrev);
    assert_eq!(state.scoped_index, 1);
}

#[test]
fn cycle_empty_noop() {
    let mut state = AppState::default();
    state.scoped_models = vec![];
    state.scoped_index = 0;

    state.update(Event::CycleModelNext);
    assert_eq!(state.scoped_index, 0);
    assert_eq!(state.current_provider, "mock");
    assert_eq!(state.current_model, "echo");
}

#[test]
fn cycle_emits_switch_model() {
    let mut state = AppState::default();
    state.scoped_models = vec![
        sm("mock", "echo", true),
        sm("openai", "gpt-4o", true),
    ];
    state.scoped_index = 0;

    state.update(Event::CycleModelNext);

    assert_eq!(state.current_provider, "openai");
    assert_eq!(state.current_model, "gpt-4o");

    let sys_msgs: Vec<_> = state
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    assert!(
        sys_msgs.iter().any(|m| m.content.contains("Switched to openai/gpt-4o")),
        "Should add system message on model switch, got: {:?}",
        sys_msgs
    );
}

#[test]
fn cycle_skips_disabled_models() {
    let mut state = AppState::default();
    state.scoped_models = vec![
        sm("mock", "echo", true),
        sm("openai", "gpt-4o", false),
        sm("anthropic", "claude-3", true),
    ];
    state.scoped_index = 0;

    state.update(Event::CycleModelNext);

    // Should skip disabled gpt-4o and land on claude-3
    assert_eq!(state.scoped_index, 2);
    assert_eq!(state.current_provider, "anthropic");
    assert_eq!(state.current_model, "claude-3");
}

#[test]
fn cycle_all_disabled_noop() {
    let mut state = AppState::default();
    state.scoped_models = vec![
        sm("mock", "echo", false),
        sm("openai", "gpt-4o", false),
    ];
    state.scoped_index = 0;

    state.update(Event::CycleModelNext);
    assert_eq!(state.scoped_index, 0);
    assert_eq!(state.current_provider, "mock");
    assert_eq!(state.current_model, "echo");
}
