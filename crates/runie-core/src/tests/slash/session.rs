use super::{exec, fresh_state, minimal_session, tmp_store, type_str, ENV_LOCK};
use crate::event::Event;
use crate::event::{DialogEvent, InputEvent};
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
fn delete_no_args_opens_form() {
    let mut state = fresh_state();
    palette_select(&mut state, "delete");

    // Should open form dialog
    assert!(state.open_dialog.is_some(), "should open dialog");
    if let Some(crate::commands::DialogState::PanelStack(stack)) = &state.open_dialog {
        let panel = stack.current().expect("should have panel");
        assert_eq!(panel.id, "delete", "should be delete form");
    } else {
        panic!("expected PanelStack dialog");
    }
}

#[test]
fn slash_command_does_not_queue() {
    let mut state = fresh_state();
    palette_select(&mut state, "session");
    assert!(
        state.agent.request_queue.is_empty(),
        "slash commands are not queued"
    );
}

#[test]
fn unknown_slash_returns_error() {
    let mut state = fresh_state();
    exec(&mut state, "/unknown");

    let sys_msgs: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    assert_eq!(sys_msgs.len(), 1);
    assert!(sys_msgs[0].content.contains("Unknown command"));
}

#[test]
fn slash_with_extra_whitespace_trimmed() {
    let mut state = fresh_state();
    palette_select(&mut state, "session");

    let sys_msgs: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    assert!(!sys_msgs.is_empty(), "trimmed slash command works");
}

#[test]
fn save_trims_whitespace() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let store = tmp_store();
    std::env::set_var("RUNIE_SESSIONS_DIR", store.dir().to_path_buf());

    let mut state = fresh_state();
    exec(&mut state, "/save  trimmed"); // Opens form with pre-filled name
    state.update(Event::submit()); // Submits the form

    // Should save with trimmed name
    let redb_path = crate::session_store::SessionStore::new(store.dir().to_path_buf()).path("trimmed");
    assert!(redb_path.exists(), "whitespace should be trimmed");

    std::env::remove_var("RUNIE_SESSIONS_DIR");
}

#[test]
fn new_clears_session_keeps_provider_model() {
    let mut state = fresh_state();
    // The configured default is different from the currently active model.
    // /new must keep the active model, not revert to the default.
    state.config.current_provider = "openai".into();
    state.config.current_model = "gpt-4o".into();
    state.config.model_source = crate::state::ModelSource::UserOverride;
    state.session.session_display_name = Some("my chat".into());

    type_str(&mut state, "hello");
    state.update(Event::submit());
    assert!(
        !state.session.messages.is_empty(),
        "should have user message"
    );

    exec(&mut state, "/new");

    assert!(
        state
            .session
            .messages
            .iter()
            .all(|m| m.role == Role::System),
        "non-system messages cleared"
    );
    assert_eq!(
        state.session.session_display_name, None,
        "display name reset"
    );
    assert_eq!(state.config.current_provider, "openai", "provider kept");
    assert_eq!(state.config.current_model, "gpt-4o", "model kept");
    let sys: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    assert!(
        sys.iter().any(|m| m.content.contains("New session")),
        "confirmation: {:?}",
        sys.last()
    );
}

#[test]
fn new_closes_open_dialog_and_clears_ui_state() {
    let mut state = fresh_state();
    palette_select(&mut state, "delete");
    assert!(state.open_dialog.is_some(), "dialog should be open");
    state.dialog_back_stack.push(crate::commands::DialogState::Welcome);
    state.login_flow = Some(crate::login_flow::LoginFlowState::default());
    state.permission_request = Some(crate::model::PermissionRequestState {
        request_id: "perm".into(),
        tool: "bash".into(),
        input: serde_json::Value::Null,
    });

    state.update(Event::RunPaletteCommand {
        name: "new".into(),
        args: "".into(),
    });

    assert!(state.open_dialog.is_none(), "open_dialog cleared");
    assert!(state.dialog_back_stack.is_empty(), "back stack cleared");
    assert!(state.login_flow.is_none(), "login_flow cleared");
    assert!(state.permission_request.is_none(), "permission_request cleared");
}

#[test]
fn history_lists_recent_inputs() {
    let mut state = fresh_state();
    for text in ["first question", "second question"] {
        type_str(&mut state, text);
        state.update(Event::submit());
    }

    exec(&mut state, "/history");

    let sys: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    let last = sys.last().expect("system message");
    assert!(
        last.content.contains("first question"),
        "lists first: {}",
        last.content
    );
    assert!(
        last.content.contains("second question"),
        "lists second: {}",
        last.content
    );
    assert!(
        last.content.contains("total"),
        "shows count: {}",
        last.content
    );
}

#[test]
fn resume_loads_most_recent_session() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let store = tmp_store();
    std::env::set_var("RUNIE_SESSIONS_DIR", store.dir().to_path_buf());

    // Save an older session
    let mut older = fresh_state();
    older.config.current_provider = "anthropic".into();
    older.config.current_model = "claude-3".into();
    older.session.messages.push(crate::model::ChatMessage {
        role: Role::User,
        content: "older".into(),
        timestamp: 1.0,
        id: "u.older".into(),
        ..Default::default()
    });
    crate::session_replay::save_session("older", &older).unwrap();

    // Save a newer session
    let mut newer = fresh_state();
    newer.config.current_provider = "openai".into();
    newer.config.current_model = "gpt-4o".into();
    newer.session.messages.push(crate::model::ChatMessage {
        role: Role::User,
        content: "newer".into(),
        timestamp: 2.0,
        id: "u.newer".into(),
        ..Default::default()
    });
    crate::session_replay::save_session("newer", &newer).unwrap();

    let mut state = fresh_state();
    exec(&mut state, "/resume");

    assert_eq!(
        state.config.current_provider, "openai",
        "loads newer provider"
    );
    assert_eq!(state.config.current_model, "gpt-4o", "loads newer model");
    assert!(
        state.session.messages.iter().any(|m| m.content == "newer"),
        "newer message loaded"
    );
    assert!(
        !state.session.messages.iter().any(|m| m.content == "older"),
        "older message not loaded"
    );

    std::env::remove_var("RUNIE_SESSIONS_DIR");
}
