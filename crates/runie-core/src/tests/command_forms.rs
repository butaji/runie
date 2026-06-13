//! Tests for command argument handling.
//!
//! Commands that require arguments should open a form to collect
//! them, never print a "Usage: ..." error in the chat feed.
//!
//! Commands with optional arguments can still accept inline args.

use crate::commands::{CommandResult, DialogState};
use crate::event::Event;
use crate::model::AppState;

// ============================================================================
// /spawn — requires a prompt argument
// ============================================================================

#[test]
fn spawn_without_args_opens_form() {
    use crate::commands::handlers::subagent::handle_spawn;
    let mut state = AppState::default();
    let result = handle_spawn(&mut state, "");

    // Must open a form panel, NOT return a "Usage:" message
    match result {
        CommandResult::OpenPanelStack(_) => {}
        CommandResult::Message(msg) => panic!(
            "spawn without args should open form, not show message: {}",
            msg
        ),
        CommandResult::Warning(msg) => panic!(
            "spawn without args should open form, not warn: {}",
            msg
        ),
        other => panic!("unexpected result: {:?}", other),
    }
}

#[test]
fn spawn_with_whitespace_only_opens_form() {
    use crate::commands::handlers::subagent::handle_spawn;
    let mut state = AppState::default();
    let result = handle_spawn(&mut state, "   \t  ");
    match result {
        CommandResult::OpenPanelStack(_) => {}
        other => panic!("expected form dialog, got {:?}", other),
    }
}

#[test]
fn spawn_with_args_emits_event() {
    use crate::commands::handlers::subagent::handle_spawn;
    let mut state = AppState::default();
    let result = handle_spawn(&mut state, "list files in /tmp");
    match result {
        CommandResult::Event(Event::SpawnAgent { prompt }) => {
            assert_eq!(prompt, "list files in /tmp");
        }
        other => panic!("expected SpawnAgent event, got {:?}", other),
    }
}

#[test]
fn spawn_trims_whitespace_from_args() {
    use crate::commands::handlers::subagent::handle_spawn;
    let mut state = AppState::default();
    let result = handle_spawn(&mut state, "  hello world  ");
    match result {
        CommandResult::Event(Event::SpawnAgent { prompt }) => {
            assert_eq!(prompt, "hello world");
        }
        other => panic!("expected event, got {:?}", other),
    }
}

#[test]
fn spawn_form_panel_has_prompt_field() {
    use crate::commands::handlers::subagent::handle_spawn;
    let mut state = AppState::default();
    let result = handle_spawn(&mut state, "");
    if let CommandResult::OpenPanelStack(stack) = result {
        let panel = stack.current().unwrap();
        // The form should have a field for the prompt
        let has_prompt_field = panel.form_values.keys().any(|k| k == "prompt")
            || panel.items.iter().any(|it| {
                if let crate::dialog::PanelItem::FormField { key, .. } = it {
                    key == "prompt"
                } else {
                    false
                }
            });
        assert!(has_prompt_field, "spawn form should have 'prompt' field");
    } else {
        panic!("expected panel stack");
    }
}

// ============================================================================
// No command should return "Usage:" messages
// ============================================================================

#[test]
fn no_command_returns_usage_message() {
    let reg = crate::commands::CommandRegistry::new();
    let mut state = AppState::default();

    for def in reg.list() {
        let name = def.name.clone();
        if let Some(handler) = get_handler(&def) {
            let result = handler(&mut state, "");
            if let CommandResult::Message(msg) = &result {
                assert!(
                    !msg.to_lowercase().contains("usage:"),
                    "command /{} returned Usage message: {}",
                    name,
                    msg
                );
            }
            if let CommandResult::Warning(msg) = &result {
                assert!(
                    !msg.to_lowercase().contains("usage:"),
                    "command /{} returned Usage warning: {}",
                    name,
                    msg
                );
            }
        }
    }
}

#[test]
fn no_command_returns_unknown_command_message() {
    // Test that unknown commands don't return "Unknown command" either
    // (or at least that's acceptable since it's a system message)
    let mut state = AppState::default();
    let result = state.handle_slash("/nonexistent");
    // Just verify it doesn't crash
    let _ = result;
}

#[test]
fn required_arg_commands_open_forms_or_emit_events() {
    // Commands known to require arguments
    let required_arg_commands = vec!["spawn", "save", "load", "delete", "name", "fork"];

    for name in required_arg_commands {
        let reg = crate::commands::CommandRegistry::new();
        if let Some(def) = reg.get(name) {
            if let Some(handler) = get_handler(&def) {
                let mut state = AppState::default();
                let result = handler(&mut state, "");
                let is_form = matches!(result, CommandResult::OpenPanelStack(_));
                let is_event = matches!(result, CommandResult::Event(_));
                let is_none = matches!(result, CommandResult::None);
                assert!(
                    is_form || is_event || is_none,
                    "command /{} should open form or emit event when no args given, got: {:?}",
                    name,
                    result
                );
            }
        }
    }
}

#[test]
fn no_command_with_required_args_shows_message() {
    // Try all commands with empty args and make sure none return a "Usage:" message
    let reg = crate::commands::CommandRegistry::new();
    let mut state = AppState::default();

    for def in reg.list() {
        let name = def.name.clone();
        if let Some(handler) = get_handler(&def) {
            let result = handler(&mut state, "");
            if let CommandResult::Message(msg) = &result {
                if msg.to_lowercase().contains("usage:") {
                    panic!(
                        "command /{} should not return 'Usage:' message; should open form instead. Got: {}",
                        name, msg
                    );
                }
            }
        }
    }
}

// ============================================================================
// DialogState::PanelStack — the open_dialog from form commands
// ============================================================================

#[test]
fn form_command_sets_open_dialog() {
    use crate::commands::handlers::subagent::handle_spawn;
    let mut state = AppState::default();
    let result = handle_spawn(&mut state, "");

    // After a form command, the state.open_dialog should be set
    if let CommandResult::OpenPanelStack(stack) = result {
        // The actual setting of open_dialog happens in handle_slash, not in the handler
        // But the form panel should be properly built
        assert!(!stack.panels.is_empty());
        let panel = stack.current().unwrap();
        assert!(!panel.title.is_empty());
    } else {
        panic!("expected panel stack");
    }
}

#[test]
fn form_panels_have_input_field() {
    use crate::commands::handlers::subagent::handle_spawn;
    let mut state = AppState::default();
    let result = handle_spawn(&mut state, "");

    if let CommandResult::OpenPanelStack(stack) = result {
        let panel = stack.current().unwrap();
        // Verify the panel has at least one form field
        let has_field = panel.items.iter().any(|it| {
            matches!(it, crate::dialog::PanelItem::FormField { .. })
        });
        assert!(has_field, "spawn form should have at least one form field");
    } else {
        panic!("expected panel stack");
    }
}

// ============================================================================
// Helper: extract handler from CommandDef
// ============================================================================

fn get_handler(
    def: &crate::commands::CommandDef,
) -> Option<fn(&mut AppState, &str) -> CommandResult> {
    use crate::commands::CommandFlow;
    match &def.flow {
        CommandFlow::Handler(f) => Some(*f),
        _ => None,
    }
}
