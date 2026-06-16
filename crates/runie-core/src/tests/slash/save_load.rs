use super::{exec, fresh_state, minimal_session, tmp_store, ENV_LOCK};
use crate::event::{InputEvent, DialogEvent};
use crate::event::Event;
use crate::model::{ChatMessage, Role};
use crate::session::Session;

/// Open palette and select a command by name
fn palette_select(state: &mut crate::model::AppState, cmd: &str) {
    state.update(InputEvent::Input('/'));
    for c in cmd.chars() {
        state.update(DialogEvent::PaletteFilter(c));
    }
    state.update(DialogEvent::PaletteSelect);
}

fn restored_session() -> Session {
    let mut session = minimal_session("restore_me");
    session.updated_at = 2.0;
    session.provider = "anthropic".into();
    session.model = "claude-3".into();
    session.messages = vec![
        ChatMessage {
            role: Role::User,
            content: "hi".into(),
            timestamp: 1.0,
            id: "req.0".into(),
            ..Default::default()
        },
        ChatMessage {
            role: Role::Assistant,
            content: "hello there".into(),
            timestamp: 2.0,
            id: "resp.0".into(),
            ..Default::default()
        },
    ];
    session
}

fn system_messages(state: &crate::model::AppState) -> Vec<&ChatMessage> {
    state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect()
}

#[test]
fn load_restores_conversation() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let store = tmp_store();
    std::env::set_var("RUNIE_SESSIONS_DIR", store.dir.clone());

    store.save("restore_me", &restored_session()).unwrap();

    let mut state = fresh_state();
    exec(&mut state, "/load restore_me");
    state.update(Event::submit());

    assert_eq!(state.session.messages.len(), 3);
    assert_eq!(state.session.messages[0].content, "hi");
    assert_eq!(state.session.messages[1].content, "hello there");
    assert_eq!(state.config.current_provider, "anthropic");
    assert_eq!(state.config.current_model, "claude-3");
    assert!(system_messages(&state)
        .iter()
        .any(|m| m.content.contains("loaded")));

    std::env::remove_var("RUNIE_SESSIONS_DIR");
}

#[test]
fn load_missing_session_shows_error() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let store = tmp_store();
    std::env::set_var("RUNIE_SESSIONS_DIR", store.dir.clone());

    let mut state = fresh_state();
    exec(&mut state, "/load nope"); // Opens form with pre-filled name
    state.update(Event::submit()); // Submits the form

    let sys_msgs: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    let last = sys_msgs.last().expect("system msg");
    assert!(
        last.content.contains("not found"),
        "user-friendly not-found: {}",
        last.content
    );
    assert!(
        last.content.contains("/sessions"),
        "should suggest /sessions: {}",
        last.content
    );

    std::env::remove_var("RUNIE_SESSIONS_DIR");
}

#[test]
fn load_no_args_opens_form() {
    let mut state = fresh_state();
    palette_select(&mut state, "load");

    // Should open form dialog
    assert!(state.open_dialog.is_some(), "should open dialog");
    if let Some(crate::commands::DialogState::PanelStack(stack)) = &state.open_dialog {
        let panel = stack.current().expect("should have panel");
        assert_eq!(panel.id, "load", "should be load form");
    } else {
        panic!("expected PanelStack dialog");
    }
}

#[test]
fn sessions_lists_saved_sessions() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let store = tmp_store();
    std::env::set_var("RUNIE_SESSIONS_DIR", store.dir.clone());

    store.save("alpha", &minimal_session("alpha")).unwrap();
    store.save("beta", &minimal_session("beta")).unwrap();

    let mut state = fresh_state();
    palette_select(&mut state, "sessions");

    let sys_msgs: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    let last = sys_msgs.last().expect("system msg");
    assert!(
        last.content.contains("alpha"),
        "lists alpha: {}",
        last.content
    );
    assert!(
        last.content.contains("beta"),
        "lists beta: {}",
        last.content
    );

    std::env::remove_var("RUNIE_SESSIONS_DIR");
}

#[test]
fn sessions_empty_shows_no_sessions() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let store = tmp_store();
    std::env::set_var("RUNIE_SESSIONS_DIR", store.dir.clone());

    let mut state = fresh_state();
    palette_select(&mut state, "sessions");

    let sys_msgs: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    let last = sys_msgs.last().expect("system msg");
    assert!(
        last.content.contains("No saved sessions"),
        "empty message: {}",
        last.content
    );

    std::env::remove_var("RUNIE_SESSIONS_DIR");
}

#[test]
fn delete_removes_session_file() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let store = tmp_store();
    std::env::set_var("RUNIE_SESSIONS_DIR", store.dir.clone());

    store.save("gone", &minimal_session("gone")).unwrap();

    let mut state = fresh_state();
    exec(&mut state, "/delete gone");
    state.update(Event::submit());

    assert!(!store.path("gone").exists(), "session file removed");

    let sys_msgs: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    let last = sys_msgs.last().expect("system msg");
    assert!(
        last.content.contains("deleted"),
        "confirmation shown: {}",
        last.content
    );

    std::env::remove_var("RUNIE_SESSIONS_DIR");
}

#[test]
fn delete_missing_session_shows_error() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let store = tmp_store();
    std::env::set_var("RUNIE_SESSIONS_DIR", store.dir.clone());

    let mut state = fresh_state();
    exec(&mut state, "/delete missing"); // Opens form with pre-filled name
    state.update(Event::submit()); // Submits the form

    let sys_msgs: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    let last = sys_msgs.last().expect("system msg");
    assert!(
        last.content.contains("not found"),
        "user-friendly not-found: {}",
        last.content
    );
    assert!(
        last.content.contains("/sessions"),
        "should suggest /sessions: {}",
        last.content
    );

    std::env::remove_var("RUNIE_SESSIONS_DIR");
}
