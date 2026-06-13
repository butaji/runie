use crate::event::Event;
use crate::model::Role;
use super::{exec, fresh_state, type_str};

#[test]
fn reset_clears_messages_and_input() {
    let mut state = fresh_state();
    type_str(&mut state, "hello");
    state.update(Event::Submit);
    state.agent.streaming = true;
    state.view.scroll = 5;

    type_str(&mut state, "/reset");
    state.update(Event::Submit);

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
    type_str(&mut state, "/reset");
    state.update(Event::Submit);
    // /reset must not change the current provider/model.
    assert_eq!(state.config.current_provider, initial_provider);
    assert_eq!(state.config.current_model, initial_model);
}

#[test]
fn help_shows_system_message() {
    let mut state = fresh_state();
    type_str(&mut state, "/help");
    state.update(Event::Submit);

    let sys_msgs: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    assert_eq!(sys_msgs.len(), 1);
    assert!(
        sys_msgs[0].content.contains("/model"),
        "help mentions /model"
    );
    assert!(sys_msgs[0].content.contains("/save"), "help mentions /save");
    assert!(sys_msgs[0].content.contains("/load"), "help mentions /load");
    assert!(
        sys_msgs[0].content.contains("/sessions"),
        "help mentions /sessions"
    );
    assert!(
        sys_msgs[0].content.contains("/delete"),
        "help mentions /delete"
    );
    assert!(
        sys_msgs[0].content.contains("/reset"),
        "help mentions /reset"
    );
    assert!(sys_msgs[0].content.contains("/help"), "help mentions /help");
}

#[test]
fn help_clears_input() {
    let mut state = fresh_state();
    type_str(&mut state, "/help");
    state.update(Event::Submit);
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
