use crate::event::Event;
use crate::model::Role;
use super::{exec, fresh_state, tmp_store, type_str, ENV_LOCK};

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
