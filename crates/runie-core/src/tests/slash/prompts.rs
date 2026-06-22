//! /prompt slash command tests.

use super::exec;
use crate::event::Event;
use crate::model::Role;
use crate::prompts::{PromptSource, PromptTemplate};
use crate::tests::fresh_state;

fn state_with_prompts() -> crate::model::AppState {
    crate::model::AppState {
        prompts: vec![
            PromptTemplate {
                name: "default".into(),
                content: "Be helpful.".into(),
                source: PromptSource::BuiltIn,
            },
            PromptTemplate {
                name: "custom".into(),
                content: "Be concise.".into(),
                source: PromptSource::BuiltIn,
            },
        ],
        ..Default::default()
    }
}

#[test]
fn prompt_custom_switches_prompt() {
    let mut state = state_with_prompts();
    exec(&mut state, "/prompt custom");
    state.update(Event::submit());

    assert_eq!(state.input.current_prompt, "custom");
    let sys: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    assert!(
        sys.iter().any(|m| m.content.contains("custom")),
        "switch confirmation: {:?}",
        sys.last()
    );
}

#[test]
fn prompt_no_args_lists_current_and_available() {
    let mut state = state_with_prompts();
    state.input.current_prompt = "custom".into();
    exec(&mut state, "/prompt");
    state.update(Event::submit());

    let sys: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    let last = sys.last().expect("system message");
    assert!(
        last.content.contains("Current prompt: custom"),
        "shows current: {}",
        last.content
    );
    assert!(
        last.content.contains("default") && last.content.contains("custom"),
        "lists available: {}",
        last.content
    );
}

#[test]
fn prompt_unknown_shows_error() {
    let mut state = state_with_prompts();
    exec(&mut state, "/prompt unknown");
    state.update(Event::submit());

    assert_ne!(state.input.current_prompt, "unknown");
    let sys: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    let last = sys.last().expect("system message");
    assert!(
        last.content.contains("not found"),
        "expected not-found: {}",
        last.content
    );
}
