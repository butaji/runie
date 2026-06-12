//! Slash command tests — ensure all /commands work as users expect
use crate::event::Event;
use crate::model::{AppState, ChatMessage, Role};
use crate::session::Store;
use std::sync::Mutex;

pub static ENV_LOCK: Mutex<()> = Mutex::new(());

pub fn fresh_state() -> AppState {
    AppState::default()
}

pub fn type_str(state: &mut AppState, text: &str) {
    for c in text.chars() {
        state.update(Event::Input(c));
    }
}

/// Set input buffer directly and submit — bypasses the command palette.
/// Use for slash commands that need arguments.
pub fn exec(state: &mut AppState, text: &str) {
    state.input.input = text.into();
    state.input.cursor_pos = text.len();
    state.update(Event::Submit);
}

pub fn tmp_store() -> Store {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("runie_slash_test_{}_{}", std::process::id(), n));
    let _ = std::fs::remove_dir_all(&dir);
    Store::new(dir)
}

#[test]
fn reset_clears_messages_and_input() {
    let mut state = fresh_state();
    type_str(&mut state, "hello");
    state.update(Event::Submit);
    state.streaming = true;
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
    assert!(!state.streaming, "streaming cleared");
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

#[test]
fn model_m3_just_model_name() {
    let mut state = fresh_state();
    state.config.current_provider = "mock".into();
    state.config.current_model = "echo".into();
    exec(&mut state, "/model m3");

    assert_eq!(state.config.current_provider, "mock");
    assert_eq!(state.config.current_model, "m3");
    let sys_msgs: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    assert_eq!(sys_msgs.len(), 1);
    assert!(
        sys_msgs[0].content.contains("Switched to mock/m3"),
        "/model m3 should work: {}",
        sys_msgs[0].content
    );
}

#[test]
fn model_leading_slash_ignored_for_model_name() {
    let mut state = fresh_state();
    state.config.current_provider = "mock".into();
    state.config.current_model = "echo".into();
    let initial_provider = state.config.current_provider.clone();
    exec(&mut state, "/model /gpt");

    assert_eq!(state.config.current_provider, initial_provider);
    assert_eq!(state.config.current_model, "gpt");
    let sys_msgs: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    assert_eq!(sys_msgs.len(), 1);
    assert!(
        sys_msgs[0].content.contains("Switched to mock/gpt"),
        "leading slash ignored: {}",
        sys_msgs[0].content
    );
}

#[test]
fn model_only_slashes_shows_usage() {
    let mut state = fresh_state();
    exec(&mut state, "/model /");

    let sys_msgs: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    assert_eq!(sys_msgs.len(), 1);
    assert!(
        sys_msgs[0].content.contains("Current:"),
        "only slashes shows usage: {}",
        sys_msgs[0].content
    );
}

#[test]
fn model_no_args_opens_selector() {
    let mut state = fresh_state();
    type_str(&mut state, "/model");
    state.update(Event::Submit);

    assert!(
        matches!(
            state.open_dialog,
            Some(crate::commands::DialogState::ModelSelector { .. })
        ),
        "no args should open model selector dialog"
    );
}

#[test]
fn save_creates_session_file() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let store = tmp_store();
    std::env::set_var("RUNIE_SESSIONS_DIR", store.dir.clone());

    let mut state = fresh_state();
    type_str(&mut state, "hello world");
    state.update(Event::Submit);
    exec(&mut state, "/save mysession"); // Opens form with pre-filled name
    state.update(Event::Submit); // Submits the form

    assert!(store.path("mysession").exists(), "session file created");

    let sys_msgs: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    let last = sys_msgs.last().expect("system msg");
    assert!(last.content.contains("saved"), "confirmation shown");

    std::env::remove_var("RUNIE_SESSIONS_DIR");
}

#[test]
fn save_preserves_messages_provider_model() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let store = tmp_store();
    std::env::set_var("RUNIE_SESSIONS_DIR", store.dir.clone());

    let mut state = fresh_state();
    state.config.current_provider = "openai".to_string();
    state.config.current_model = "gpt-4o".to_string();
    type_str(&mut state, "test message");
    state.update(Event::Submit);
    exec(&mut state, "/save preserved"); // Opens form with pre-filled name
    state.update(Event::Submit); // Submits the form

    let loaded = store.load("preserved").unwrap();
    assert_eq!(loaded.provider, "openai");
    assert_eq!(loaded.model, "gpt-4o");
    assert_eq!(loaded.messages.len(), 1);
    assert_eq!(loaded.messages[0].content, "test message");
    assert_eq!(loaded.messages[0].role, Role::User);

    std::env::remove_var("RUNIE_SESSIONS_DIR");
}

#[test]
fn save_no_args_opens_form() {
    let mut state = fresh_state();
    type_str(&mut state, "/save");
    state.update(Event::Submit);

    // Should open form dialog
    assert!(state.open_dialog.is_some(), "should open dialog");
    if let Some(crate::commands::DialogState::PanelStack(stack)) = &state.open_dialog {
        let panel = stack.current().expect("should have panel");
        assert_eq!(panel.id, "save", "should be save form");
    } else {
        panic!("expected PanelStack dialog");
    }
}

#[test]
fn load_restores_conversation() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let store = tmp_store();
    std::env::set_var("RUNIE_SESSIONS_DIR", store.dir.clone());

    store
        .save(
            "restore_me",
            &crate::session::Session {
                name: "restore_me".to_string(),
                created_at: 1.0,
                updated_at: 2.0,
                messages: vec![
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
                ],
                provider: "anthropic".into(),
                model: "claude-3".into(),
                theme_name: "runie".into(),
                thinking_level: crate::model::ThinkingLevel::Off,
                read_only: false,
                display_name: None,
                session_tree: None,
            },
        )
        .unwrap();

    let mut state = fresh_state();
    exec(&mut state, "/load restore_me"); // Opens form with pre-filled name
    state.update(Event::Submit); // Submits the form

    assert_eq!(state.session.messages.len(), 3); // 2 loaded + 1 system confirmation
    assert_eq!(state.session.messages[0].content, "hi");
    assert_eq!(state.session.messages[1].content, "hello there");
    assert_eq!(state.config.current_provider, "anthropic");
    assert_eq!(state.config.current_model, "claude-3");

    let sys_msgs: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    assert!(sys_msgs.iter().any(|m| m.content.contains("loaded")));

    std::env::remove_var("RUNIE_SESSIONS_DIR");
}

#[test]
fn load_missing_session_shows_error() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let store = tmp_store();
    std::env::set_var("RUNIE_SESSIONS_DIR", store.dir.clone());

    let mut state = fresh_state();
    exec(&mut state, "/load nope"); // Opens form with pre-filled name
    state.update(Event::Submit); // Submits the form

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
    type_str(&mut state, "/load");
    state.update(Event::Submit);

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

    store
        .save(
            "alpha",
            &crate::session::Session {
                name: "alpha".to_string(),
                created_at: 1.0,
                updated_at: 1.0,
                messages: vec![],
                provider: "mock".into(),
                model: "echo".into(),
                theme_name: "runie".into(),
                thinking_level: crate::model::ThinkingLevel::Off,
                read_only: false,
                display_name: None,
                session_tree: None,
            },
        )
        .unwrap();
    store
        .save(
            "beta",
            &crate::session::Session {
                name: "beta".to_string(),
                created_at: 1.0,
                updated_at: 1.0,
                messages: vec![],
                provider: "mock".into(),
                model: "echo".into(),
                theme_name: "runie".into(),
                thinking_level: crate::model::ThinkingLevel::Off,
                read_only: false,
                display_name: None,
                session_tree: None,
            },
        )
        .unwrap();

    let mut state = fresh_state();
    type_str(&mut state, "/sessions");
    state.update(Event::Submit);

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
    type_str(&mut state, "/sessions");
    state.update(Event::Submit);

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

    store
        .save(
            "gone",
            &crate::session::Session {
                name: "gone".to_string(),
                created_at: 1.0,
                updated_at: 1.0,
                messages: vec![],
                provider: "mock".into(),
                model: "echo".into(),
                theme_name: "runie".into(),
                thinking_level: crate::model::ThinkingLevel::Off,
                read_only: false,
                display_name: None,
                session_tree: None,
            },
        )
        .unwrap();

    let mut state = fresh_state();
    exec(&mut state, "/delete gone"); // Opens form with pre-filled name
    state.update(Event::Submit); // Submits the form

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
    state.update(Event::Submit); // Submits the form

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
fn delete_no_args_opens_form() {
    let mut state = fresh_state();
    type_str(&mut state, "/delete");
    state.update(Event::Submit);

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
    type_str(&mut state, "/help");
    state.update(Event::Submit);
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
    type_str(&mut state, "  /help  ");
    state.update(Event::Submit);

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
    std::env::set_var("RUNIE_SESSIONS_DIR", store.dir.clone());

    let mut state = fresh_state();
    exec(&mut state, "/save  trimmed"); // Opens form with pre-filled name
    state.update(Event::Submit); // Submits the form

    // Should save with trimmed name
    assert!(
        store.path("trimmed").exists(),
        "whitespace should be trimmed"
    );

    std::env::remove_var("RUNIE_SESSIONS_DIR");
}
