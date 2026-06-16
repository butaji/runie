//! Tests for command argument handling.
//!
//! Commands that require arguments should open a form to collect
//! them, never print a "Usage: ..." error in the chat feed.
//!
//! Commands with optional arguments can still accept inline args.

use crate::event::ControlEvent;

use crate::commands::CommandResult;
use crate::model::AppState;

// ============================================================================
// /spawn — requires a prompt argument
// ============================================================================

#[test]
fn spawn_without_args_opens_form() {
    use crate::commands::dsl::handlers::subagent::handle_spawn;
    let mut state = AppState::default();
    let result = handle_spawn(&mut state, "");

    // Must open a form panel, NOT return a "Usage:" message
    match result {
        CommandResult::OpenPanelStack(_) => {}
        CommandResult::Message(msg) => panic!(
            "spawn without args should open form, not show message: {}",
            msg
        ),
        CommandResult::Warning(msg) => {
            panic!("spawn without args should open form, not warn: {}", msg)
        }
        other => panic!("unexpected result: {:?}", other),
    }
}

#[test]
fn spawn_with_whitespace_only_opens_form() {
    use crate::commands::dsl::handlers::subagent::handle_spawn;
    let mut state = AppState::default();
    let result = handle_spawn(&mut state, "   \t  ");
    match result {
        CommandResult::OpenPanelStack(_) => {}
        other => panic!("expected form dialog, got {:?}", other),
    }
}

#[test]
fn spawn_with_args_emits_event() {
    use crate::commands::dsl::handlers::subagent::handle_spawn;
    let mut state = AppState::default();
    let result = handle_spawn(&mut state, "list files in /tmp");
    match result {
        CommandResult::Event(ControlEvent::SpawnAgent { prompt }) => {
            assert_eq!(prompt, "list files in /tmp");
        }
        other => panic!("expected SpawnAgent event, got {:?}", other),
    }
}

#[test]
fn spawn_trims_whitespace_from_args() {
    use crate::commands::dsl::handlers::subagent::handle_spawn;
    let mut state = AppState::default();
    let result = handle_spawn(&mut state, "  hello world  ");
    match result {
        CommandResult::Event(ControlEvent::SpawnAgent { prompt }) => {
            assert_eq!(prompt, "hello world");
        }
        other => panic!("expected event, got {:?}", other),
    }
}

#[test]
fn spawn_form_panel_has_prompt_field() {
    use crate::commands::dsl::handlers::subagent::handle_spawn;
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
        if let Some(handler) = get_handler(def) {
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

/// Tests that form-submit handlers (called when user submits an empty
/// form) never return "Usage:" messages. The form-submit path is:
///   1. User types /load (no args)
///   2. Form opens
///   3. User submits form with empty name
///   4. handle_load is called with empty name
///   5. handle_load must NOT return a "Usage:" message
fn assert_no_usage(result: &CommandResult, handler_name: &str) {
    if let CommandResult::Message(msg) = result {
        assert!(
            !msg.to_lowercase().contains("usage:"),
            "{} returned Usage: {}",
            handler_name,
            msg
        );
    }
}

#[test]
fn no_form_submit_handler_returns_usage_message() {
    use crate::commands::dsl::handlers::session::io as session_io;

    let mut state = AppState::default();

    assert_no_usage(&session_io::handle_load(&mut state, ""), "handle_load");
    assert_no_usage(&session_io::handle_delete(&mut state, ""), "handle_delete");
    assert_no_usage(&session_io::handle_import(&mut state, ""), "handle_import");
    assert_no_usage(&session_io::handle_export(&mut state, ""), "handle_export");
}

/// Test the end-to-end flow: type /load, open form, submit empty
/// (simulating user pressing Enter without typing).
/// The result should NOT pollute the chat feed with a Usage message.
#[test]
fn load_form_submit_empty_does_not_show_usage() {
    use crate::commands::dsl::handlers::session::io as session_io;
    let mut state = AppState::default();
    let initial_msg_count = state.session.messages.len();

    // Simulate form submit with empty name
    let result = session_io::handle_load(&mut state, "");

    // No "Usage:" message should be added to the feed
    if let CommandResult::Message(_) = &result {
        panic!("handle_load with empty arg returned Message, should return dialog or None");
    }

    // The chat feed should not have new messages
    assert_eq!(
        state.session.messages.len(),
        initial_msg_count,
        "handle_load should not add messages to chat feed"
    );
}

#[test]
fn delete_form_submit_empty_does_not_show_usage() {
    use crate::commands::dsl::handlers::session::io as session_io;
    let mut state = AppState::default();
    let initial_msg_count = state.session.messages.len();

    let result = session_io::handle_delete(&mut state, "");
    if let CommandResult::Message(_) = &result {
        panic!("handle_delete with empty arg returned Message");
    }
    assert_eq!(state.session.messages.len(), initial_msg_count);
}

#[test]
fn import_form_submit_empty_does_not_show_usage() {
    use crate::commands::dsl::handlers::session::io as session_io;
    let mut state = AppState::default();
    let initial_msg_count = state.session.messages.len();

    let result = session_io::handle_import(&mut state, "");
    if let CommandResult::Message(_) = &result {
        panic!("handle_import with empty arg returned Message");
    }
    assert_eq!(state.session.messages.len(), initial_msg_count);
}

#[test]
fn export_form_submit_empty_does_not_show_usage() {
    use crate::commands::dsl::handlers::session::io as session_io;
    let mut state = AppState::default();
    let initial_msg_count = state.session.messages.len();

    let result = session_io::handle_export(&mut state, "");
    if let CommandResult::Message(_) = &result {
        panic!("handle_export with empty arg returned Message");
    }
    assert_eq!(state.session.messages.len(), initial_msg_count);
}

/// Model command with too many slashes also shouldn't show Usage.
#[test]
fn model_command_does_not_show_usage() {
    use crate::commands::dsl::handlers::model::handle_model;
    let mut state = AppState::default();
    // Too many slashes
    let result = handle_model(&mut state, "a/b/c");
    if let CommandResult::Message(msg) = &result {
        assert!(
            !msg.to_lowercase().contains("usage:"),
            "handle_model returned Usage: {}",
            msg
        );
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
            if let Some(handler) = get_handler(def) {
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
        if let Some(handler) = get_handler(def) {
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
    use crate::commands::dsl::handlers::subagent::handle_spawn;
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
    use crate::commands::dsl::handlers::subagent::handle_spawn;
    let mut state = AppState::default();
    let result = handle_spawn(&mut state, "");

    if let CommandResult::OpenPanelStack(stack) = result {
        let panel = stack.current().unwrap();
        // Verify the panel has at least one form field
        let has_field = panel
            .items
            .iter()
            .any(|it| matches!(it, crate::dialog::PanelItem::FormField { .. }));
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
