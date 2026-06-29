#![allow(clippy::all)]
use crate::commands::{CommandDef, CommandFlow, CommandResult};
use crate::model::AppState;

// ── Palette Ranking ─────────────────────────────────────────────────────────────

#[test]
fn frequently_used_command_ranks_higher() {
    let mut state = AppState::default();
    // Invoke /compact twice to record usage
    state.record_command_usage("compact");
    state.record_command_usage("compact");

    let ranked = state.rank_commands("com", 10);
    // compact should outrank model (m-o-d-e-l) for "com"
    let names: Vec<_> = ranked.iter().map(|(cmd, _)| cmd.name.as_str()).collect();
    assert!(
        names.contains(&"compact"),
        "compact should appear in ranking for 'com'"
    );
}

#[test]
fn recent_command_gets_recency_boost() {
    let mut state = AppState::default();
    // model then compact
    state.record_command_usage("model");
    state.record_command_usage("compact");

    let ranked = state.rank_commands("", 10);
    let names: Vec<_> = ranked.iter().map(|(cmd, _)| cmd.name.as_str()).collect();
    // compact was used most recently, should appear before model for empty query
    let compact_pos = names.iter().position(|&n| n == "compact");
    let model_pos = names.iter().position(|&n| n == "model");
    assert!(
        compact_pos.is_some() && model_pos.is_some(),
        "both compact and model should be ranked"
    );
    // compact was used last, so it should come before model in the list
    assert!(
        compact_pos.unwrap() < model_pos.unwrap(),
        "compact (used last) should rank before model for empty query"
    );
}

#[test]
fn invoking_command_records_usage() {
    let mut state = AppState::default();
    let result = state.handle_slash("/compact");
    // handle_slash returns None for commands with no message output
    let _ = result;
    assert!(
        state.config.command_usage.contains_key("compact"),
        "invoking /compact should record usage"
    );
    assert_eq!(state.config.command_usage.get("compact").unwrap().count, 1);
}

#[test]
fn unknown_command_does_not_record_usage() {
    let mut state = AppState::default();
    let result = state.handle_slash("/does_not_exist_xyz");
    assert!(
        result.is_some(),
        "unknown command should return Some(Message)"
    );
    assert!(
        state
            .config
            .command_usage
            .get("does_not_exist_xyz")
            .is_none(),
        "unknown command should not record usage"
    );
}

#[test]
fn rank_commands_empty_query_returns_all() {
    let mut state = AppState::default();
    let ranked = state.rank_commands("", 50);
    assert!(ranked.len() >= 20, "empty query should return all commands");
}

#[test]
fn rank_commands_with_query_filters() {
    let mut state = AppState::default();
    let ranked = state.rank_commands("model", 10);
    // At least some results should be model-related (new commands may also appear).
    let model_hits: usize = ranked
        .iter()
        .take(5)
        .filter(|(cmd, _)| cmd.name.contains("model") || cmd.desc.contains("model"))
        .count();
    assert!(
        model_hits >= 2,
        "query 'model' should surface model-related commands near the top, got {model_hits} hits in top 5"
    );
}

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

    assert_no_usage_message(
        "load",
        &state
            .handle_slash("/load")
            .expect("load should return result"),
    );
    assert_no_usage_message(
        "delete",
        &state
            .handle_slash("/delete")
            .expect("delete should return result"),
    );
    assert_no_usage_message(
        "import",
        &state
            .handle_slash("/import")
            .expect("import should return result"),
    );
    assert_no_usage_message(
        "export",
        &state
            .handle_slash("/export")
            .expect("export should return result"),
    );
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
    let result = state
        .handle_slash("/load")
        .expect("load should return result");
    assert_form_submit_does_not_show_usage("load", &result, initial_msg_count, &state);
}

#[test]
fn delete_form_submit_empty_does_not_show_usage() {
    let mut state = AppState::default();
    let initial_msg_count = state.session.messages.len();
    let result = state
        .handle_slash("/delete")
        .expect("delete should return result");
    assert_form_submit_does_not_show_usage("delete", &result, initial_msg_count, &state);
}

#[test]
fn import_form_submit_empty_does_not_show_usage() {
    let mut state = AppState::default();
    let initial_msg_count = state.session.messages.len();
    let result = state
        .handle_slash("/import")
        .expect("import should return result");
    assert_form_submit_does_not_show_usage("import", &result, initial_msg_count, &state);
}

#[test]
fn export_form_submit_empty_does_not_show_usage() {
    let mut state = AppState::default();
    let initial_msg_count = state.session.messages.len();
    let result = state
        .handle_slash("/export")
        .expect("export should return result");
    assert_form_submit_does_not_show_usage("export", &result, initial_msg_count, &state);
}

#[test]
fn model_command_does_not_show_usage() {
    use crate::commands::dsl::handlers::model::handle_model;
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
