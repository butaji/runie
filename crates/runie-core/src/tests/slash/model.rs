use super::{exec, fresh_state, tmp_store, type_str, ENV_LOCK};
use crate::event::Event;
use crate::model::Role;

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
