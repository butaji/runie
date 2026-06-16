//! Theme slash command tests

use crate::event::{InputEvent, ControlEvent, ModelConfigEvent, SystemEvent, DialogEvent, ScrollEvent, AgentEvent, SessionEvent, EditEvent, CommandEvent, DurableCoreEvent};
use super::slash::{exec, fresh_state, tmp_store, type_str, ENV_LOCK};
use crate::event::Event;
use crate::model::Role;

/// Open palette and select a command by name
fn palette_select(state: &mut crate::model::AppState, cmd: &str) {
    state.update(Event::Input(InputEvent::Input('/')));
    for c in cmd.chars() {
        state.update(Event::Dialog(DialogEvent::PaletteFilter(c)));
    }
    state.update(Event::Dialog(DialogEvent::PaletteSelect));
}

#[test]
fn theme_switch_updates_state() {
    let mut state = fresh_state();
    exec(&mut state, "/theme dracula");

    assert_eq!(state.config.theme_name, "dracula");
    let sys_msgs: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    assert_eq!(sys_msgs.len(), 1);
    assert!(
        sys_msgs[0].content.contains("Theme switched to 'dracula'"),
        "should confirm theme switch: {}",
        sys_msgs[0].content
    );
}

#[test]
fn theme_invalid_shows_fallback_warning() {
    let mut state = fresh_state();
    exec(&mut state, "/theme not-a-real-theme");

    assert_eq!(state.config.theme_name, "not-a-real-theme");
    let sys_msgs: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    assert_eq!(sys_msgs.len(), 1);
    assert!(
        sys_msgs[0].content.contains("not found"),
        "should warn about fallback: {}",
        sys_msgs[0].content
    );
    assert!(
        sys_msgs[0].content.contains("runie"),
        "should mention fallback: {}",
        sys_msgs[0].content
    );
}

#[test]
fn theme_no_args_opens_selector_dialog() {
    let mut state = fresh_state();
    palette_select(&mut state, "theme");

    // No system message — instead a panel dialog should be open
    let sys_msgs: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    assert_eq!(
        sys_msgs.len(),
        0,
        "/theme with no args should not emit a system message"
    );

    let dialog = state
        .open_dialog
        .as_ref()
        .expect("theme selector dialog should be open");
    assert!(
        matches!(dialog, crate::commands::DialogState::PanelStack(stack) if stack.current().unwrap().id == "theme"),
        "should open theme panel dialog"
    );
}

#[test]
fn theme_persisted_in_session() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let store = tmp_store();
    std::env::set_var("RUNIE_SESSIONS_DIR", store.dir.clone());

    let mut state = fresh_state();
    state.config.theme_name = "nord".to_string();
    exec(&mut state, "/save themed"); // Opens form with pre-filled name
    state.update(Event::submit()); // Submits the form

    let loaded = store.load("themed").unwrap();
    assert_eq!(loaded.theme_name, "nord");

    std::env::remove_var("RUNIE_SESSIONS_DIR");
}
