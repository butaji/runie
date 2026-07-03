#![allow(clippy::all)]
use super::{exec, minimal_session, tmp_store};
use crate::commands::DialogKind;
use crate::message::Part;
use crate::model::{ChatMessage, Role};
use crate::session::replay::save_snapshot;
use crate::session::Session;
use crate::tests::fresh_state;
use crate::Event;
use runie_testing::with_env;

/// Open palette and select a command by name
fn palette_select(state: &mut crate::model::AppState, cmd: &str) {
    state.update(crate::Event::Input('/'));
    for c in cmd.chars() {
        state.update(crate::Event::PaletteFilter(c));
    }
    state.update(crate::Event::PaletteSelect);
}

fn restored_session() -> Session {
    let mut session = minimal_session("restore_me");
    session.updated_at = 2.0;
    session.provider = "anthropic".into();
    session.model = "claude-3".into();
    session.messages = vec![
        ChatMessage {
            role: Role::User,
            timestamp: 1.0,
            id: "req.0".into(),
            parts: vec![Part::Text {
                content: "hi".into(),
            }],
            ..Default::default()
        },
        ChatMessage {
            role: Role::Assistant,
            timestamp: 2.0,
            id: "resp.0".into(),
            parts: vec![Part::Text {
                content: "hello there".into(),
            }],
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
    with_env(|env| {
        let store = tmp_store();
        env.set(
            "RUNIE_SESSIONS_DIR",
            store.dir().to_path_buf().to_str().unwrap_or("/tmp"),
        );

        save_snapshot("restore_me", &restored_session()).unwrap();

        let mut state = fresh_state();
        exec(&mut state, "/load restore_me");
        state.update(Event::submit());

        assert_eq!(state.session.messages.len(), 3);
        assert_eq!(state.session.messages[0].content(), "hi");
        assert_eq!(state.session.messages[1].content(), "hello there");
        assert_eq!(state.config.current_provider, "anthropic");
        assert_eq!(state.config.current_model, "claude-3");
        assert!(system_messages(&state)
            .iter()
            .any(|m| m.content().contains("loaded")));
    });
}

#[test]
fn load_missing_session_shows_error() {
    with_env(|env| {
        let store = tmp_store();
        env.set(
            "RUNIE_SESSIONS_DIR",
            store.dir().to_path_buf().to_str().unwrap_or("/tmp"),
        );

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
            last.content().contains("not found"),
            "user-friendly not-found: {}",
            last.content()
        );
        assert!(
            last.content().contains("/sessions"),
            "should suggest /sessions: {}",
            last.content()
        );
    });
}

#[test]
fn load_no_args_opens_form() {
    let mut state = fresh_state();
    palette_select(&mut state, "load");

    // Should open form dialog
    assert!(state.open_dialog.is_some(), "should open dialog");
    if let Some(crate::commands::DialogState::Active {
        kind: DialogKind::Generic,
        panels: stack,
    }) = &state.open_dialog
    {
        let panel = stack.current().expect("should have panel");
        assert_eq!(panel.id, "load", "should be load form");
    } else {
        panic!("expected PanelStack dialog");
    }
}

#[test]
fn sessions_lists_saved_sessions() {
    with_env(|env| {
        let store = tmp_store();
        env.set(
            "RUNIE_SESSIONS_DIR",
            store.dir().to_path_buf().to_str().unwrap_or("/tmp"),
        );

        save_snapshot("alpha", &minimal_session("alpha")).unwrap();
        save_snapshot("beta", &minimal_session("beta")).unwrap();

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
            last.content().contains("alpha"),
            "lists alpha: {}",
            last.content()
        );
        assert!(
            last.content().contains("beta"),
            "lists beta: {}",
            last.content()
        );
    });
}

#[test]
fn sessions_empty_shows_no_sessions() {
    with_env(|env| {
        let store = tmp_store();
        env.set(
            "RUNIE_SESSIONS_DIR",
            store.dir().to_path_buf().to_str().unwrap_or("/tmp"),
        );

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
            last.content().contains("No saved sessions"),
            "empty message: {}",
            last.content()
        );
    });
}

#[test]
fn delete_removes_session_file() {
    with_env(|env| {
        let store = tmp_store();
        env.set(
            "RUNIE_SESSIONS_DIR",
            store.dir().to_path_buf().to_str().unwrap_or("/tmp"),
        );

        save_snapshot("gone", &minimal_session("gone")).unwrap();

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
            last.content().contains("deleted"),
            "confirmation shown: {}",
            last.content()
        );
    });
}

#[test]
fn delete_missing_session_shows_error() {
    with_env(|env| {
        let store = tmp_store();
        env.set(
            "RUNIE_SESSIONS_DIR",
            store.dir().to_path_buf().to_str().unwrap_or("/tmp"),
        );

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
            last.content().contains("not found"),
            "user-friendly not-found: {}",
            last.content()
        );
        assert!(
            last.content().contains("/sessions"),
            "should suggest /sessions: {}",
            last.content()
        );
    });
}
