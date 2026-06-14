use crate::commands::handlers::session::io as session_io;
use crate::commands::{CommandDef, CommandFlow, CommandResult};
use crate::model::AppState;

fn get_handler(def: &CommandDef) -> Option<fn(&mut AppState, &str) -> CommandResult> {
    match &def.flow {
        CommandFlow::Handler(f) => Some(*f),
        _ => None,
    }
}

fn assert_no_usage_message(name: &str, result: &CommandResult) {
    if let CommandResult::Message(msg) = result {
        assert!(
            !msg.to_lowercase().contains("usage:"),
            "command /{} returned Usage message: {}",
            name,
            msg
        );
    }
    if let CommandResult::Warning(msg) = result {
        assert!(
            !msg.to_lowercase().contains("usage:"),
            "command /{} returned Usage warning: {}",
            name,
            msg
        );
    }
}

#[test]
fn no_command_returns_usage_message() {
    let reg = crate::commands::CommandRegistry::new();
    let mut state = AppState::default();

    for def in reg.list() {
        let name = def.name.clone();
        if let Some(handler) = get_handler(def) {
            let result = handler(&mut state, "");
            assert_no_usage_message(&name, &result);
        }
    }
}

#[test]
fn no_form_submit_handler_returns_usage_message() {
    let mut state = AppState::default();

    assert_no_usage_message("load", &session_io::handle_load(&mut state, ""));
    assert_no_usage_message("delete", &session_io::handle_delete(&mut state, ""));
    assert_no_usage_message("import", &session_io::handle_import(&mut state, ""));
    assert_no_usage_message("export", &session_io::handle_export(&mut state, ""));
}

fn assert_form_submit_does_not_show_usage(
    name: &str,
    result: &CommandResult,
    initial_count: usize,
    state: &AppState,
) {
    if let CommandResult::Message(_) = result {
        panic!(
            "handle_{} with empty arg returned Message, should return dialog or None",
            name
        );
    }
    assert_eq!(
        state.session.messages.len(),
        initial_count,
        "handle_{} should not add messages to chat feed",
        name
    );
}

#[test]
fn load_form_submit_empty_does_not_show_usage() {
    let mut state = AppState::default();
    let initial_msg_count = state.session.messages.len();
    let result = session_io::handle_load(&mut state, "");
    assert_form_submit_does_not_show_usage("load", &result, initial_msg_count, &state);
}

#[test]
fn delete_form_submit_empty_does_not_show_usage() {
    let mut state = AppState::default();
    let initial_msg_count = state.session.messages.len();
    let result = session_io::handle_delete(&mut state, "");
    assert_form_submit_does_not_show_usage("delete", &result, initial_msg_count, &state);
}

#[test]
fn import_form_submit_empty_does_not_show_usage() {
    let mut state = AppState::default();
    let initial_msg_count = state.session.messages.len();
    let result = session_io::handle_import(&mut state, "");
    assert_form_submit_does_not_show_usage("import", &result, initial_msg_count, &state);
}

#[test]
fn export_form_submit_empty_does_not_show_usage() {
    let mut state = AppState::default();
    let initial_msg_count = state.session.messages.len();
    let result = session_io::handle_export(&mut state, "");
    assert_form_submit_does_not_show_usage("export", &result, initial_msg_count, &state);
}

#[test]
fn model_command_does_not_show_usage() {
    use crate::commands::handlers::model::handle_model;
    let mut state = AppState::default();
    let result = handle_model(&mut state, "a/b/c");
    assert_no_usage_message("model", &result);
}

#[test]
fn no_command_returns_unknown_command_message() {
    let mut state = AppState::default();
    let result = state.handle_slash("/nonexistent");
    let _ = result;
}

#[test]
fn required_arg_commands_open_forms_or_emit_events() {
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
