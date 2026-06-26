use crate::commands::{CommandResult, DialogState};
use crate::config::ModelProvider;
use crate::model::AppState;
use crate::Event;

use super::{exec_handler, run_slash};

fn seed_provider(state: &mut AppState, name: &str, models: Vec<String>) {
    state.config_mut().model_providers_mut().insert(
        name.into(),
        ModelProvider {
            provider_type: None,
            base_url: String::new(),
            api_key: String::new(),
            models,
        },
    );
}

#[test]
fn handler_model_switches() {
    let mut state = AppState::default();
    state.config.current_provider = "openai".into();
    seed_provider(&mut state, "openai", vec!["gpt-4o".into()]);
    let result = exec_handler(&mut state, "model", "gpt-4o");
    assert_eq!(state.config.current_model, "gpt-4o");
    assert!(matches!(result, CommandResult::Message(_)));
}

#[test]
fn handler_help_opens_reference_panel() {
    let mut state = AppState::default();
    let result = exec_handler(&mut state, "help", "");
    assert!(
        matches!(result, CommandResult::OpenPanelStack(_)),
        "help should open a reference panel, got {:?}",
        result
    );
}

#[test]
fn help_panel_lists_commands() {
    let mut state = AppState::default();
    run_slash(&mut state, "/help");
    let stack = match &state.open_dialog {
        Some(DialogState::PanelStack(s)) => s,
        other => panic!("expected PanelStack, got {:?}", other),
    };
    let panel = stack.current().expect("panel should exist");
    let has_commands = panel.items.iter().any(|i| match i {
        crate::dialog::PanelItem::Command { name, .. } => name == "quit",
        crate::dialog::PanelItem::Action { label, .. } => label.contains("/quit"),
        _ => false,
    });
    assert!(has_commands, "help panel should list /quit command");
}

#[test]
fn handler_quit_sets_flag() {
    let mut state = AppState::default();
    let result = exec_handler(&mut state, "quit", "");
    assert!(matches!(result, CommandResult::Event(crate::Event::Quit)));
    state.update(crate::Event::Quit);
    assert!(state.should_quit);
}

#[test]
fn unknown_command_returns_error() {
    let mut state = AppState::default();
    let result = state.handle_slash("/foo");
    assert!(matches!(result, Some(CommandResult::Message(msg)) if msg.contains("Unknown command")));
}
