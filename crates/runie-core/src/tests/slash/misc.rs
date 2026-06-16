use super::{exec, fresh_state, type_str};
use crate::event::{InputEvent, DialogEvent};
use crate::event::Event;
use crate::model::Role;

/// Open palette and select a command by name
fn palette_select(state: &mut crate::model::AppState, cmd: &str) {
    state.update(InputEvent::Input('/'));
    for c in cmd.chars() {
        state.update(DialogEvent::PaletteFilter(c));
    }
    state.update(DialogEvent::PaletteSelect);
}

#[test]
fn reset_clears_messages_and_input() {
    let mut state = fresh_state();
    type_str(&mut state, "hello");
    state.update(Event::submit());
    state.agent.streaming = true;
    state.view.scroll = 5;

    palette_select(&mut state, "reset");

    let sys_msgs: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    assert_eq!(sys_msgs.len(), 1, "reset adds confirmation");
    assert!(
        sys_msgs[0].content.contains("State cleared"),
        "reset confirmation: {}",
        sys_msgs[0].content
    );
    assert!(state.input.input.is_empty(), "input cleared");
    assert!(!state.agent.streaming, "streaming cleared");
    assert_eq!(state.view.scroll, 0, "scroll reset");
}

#[test]
fn reset_keeps_default_provider() {
    let mut state = fresh_state();
    let initial_provider = state.config.current_provider.clone();
    let initial_model = state.config.current_model.clone();
    palette_select(&mut state, "reset");
    // /reset must not change the current provider/model.
    assert_eq!(state.config.current_provider, initial_provider);
    assert_eq!(state.config.current_model, initial_model);
}

#[test]
fn help_opens_reference_panel() {
    let mut state = fresh_state();
    palette_select(&mut state, "help");

    assert!(
        matches!(
            state.open_dialog,
            Some(crate::commands::DialogState::PanelStack(_))
        ),
        "/help should open the reference panel"
    );
}

#[test]
fn help_clears_input() {
    let mut state = fresh_state();
    palette_select(&mut state, "help");
    assert!(state.input.input.is_empty());
}

#[test]
fn model_switches_provider_and_model() {
    let mut state = fresh_state();
    exec(&mut state, "/model openai/gpt-4o");

    assert_eq!(state.config.current_provider, "openai");
    assert_eq!(state.config.current_model, "gpt-4o");
}

#[test]
fn model_shows_confirmation_message() {
    let mut state = fresh_state();
    exec(&mut state, "/model anthropic/claude-3-sonnet");

    let sys_msgs: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    assert_eq!(sys_msgs.len(), 1);
    assert!(sys_msgs[0].content.contains("anthropic/claude-3-sonnet"));
}

#[test]
fn model_just_model_name_keeps_provider() {
    let mut state = fresh_state();
    state.config.current_provider = "mock".into();
    state.config.current_model = "echo".into();
    exec(&mut state, "/model openai");

    assert_eq!(state.config.current_provider, "mock");
    assert_eq!(state.config.current_model, "openai");
    let sys_msgs: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    assert_eq!(sys_msgs.len(), 1);
    assert!(
        sys_msgs[0].content.contains("Switched to mock/openai"),
        "openai without provider keeps current provider: {}",
        sys_msgs[0].content
    );
}
