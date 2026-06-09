//! Theme slash command tests
use crate::model::Role;
use crate::event::Event;
use super::slash::{ENV_LOCK, fresh_state, type_str, tmp_store};

#[test]
fn theme_switch_updates_state() {
    let mut state = fresh_state();
    type_str(&mut state, "/theme dracula");
    state.update(Event::Submit);

    assert_eq!(state.config.theme_name, "dracula");
    let sys_msgs: Vec<_> = state.session.messages.iter().filter(|m| m.role == Role::System).collect();
    assert_eq!(sys_msgs.len(), 1);
    assert!(sys_msgs[0].content.contains("Theme switched to 'dracula'"), "should confirm theme switch: {}", sys_msgs[0].content);
}

#[test]
fn theme_invalid_shows_fallback_warning() {
    let mut state = fresh_state();
    type_str(&mut state, "/theme not-a-real-theme");
    state.update(Event::Submit);

    assert_eq!(state.config.theme_name, "not-a-real-theme");
    let sys_msgs: Vec<_> = state.session.messages.iter().filter(|m| m.role == Role::System).collect();
    assert_eq!(sys_msgs.len(), 1);
    assert!(sys_msgs[0].content.contains("not found"), "should warn about fallback: {}", sys_msgs[0].content);
    assert!(sys_msgs[0].content.contains("runie"), "should mention fallback: {}", sys_msgs[0].content);
}

#[test]
fn theme_no_args_opens_selector_dialog() {
    let mut state = fresh_state();
    type_str(&mut state, "/theme");
    state.update(Event::Submit);

    // No system message — instead a panel dialog should be open
    let sys_msgs: Vec<_> = state.session.messages.iter().filter(|m| m.role == Role::System).collect();
    assert_eq!(sys_msgs.len(), 0, "/theme with no args should not emit a system message");

    let dialog = state.open_dialog.as_ref().expect("theme selector dialog should be open");
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
    type_str(&mut state, "/save themed");
    state.update(Event::Submit);

    let loaded = store.load("themed").unwrap();
    assert_eq!(loaded.theme_name, "nord");

    std::env::remove_var("RUNIE_SESSIONS_DIR");
}
