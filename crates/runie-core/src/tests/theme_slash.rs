//! Theme slash command tests

use super::slash::{exec, tmp_store, ENV_LOCK};
use crate::commands::DialogKind;
use crate::model::Role;
use crate::tests::fresh_state;
use crate::Event;

/// Open palette and select a command by name
fn palette_select(state: &mut crate::model::AppState, cmd: &str) {
    state.update(crate::Event::Input('/'));
    for c in cmd.chars() {
        state.update(crate::Event::PaletteFilter(c));
    }
    state.update(crate::Event::PaletteSelect);
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
        sys_msgs[0]
            .content()
            .contains("Theme switched to 'dracula'"),
        "should confirm theme switch: {}",
        sys_msgs[0].content()
    );
}

#[test]
fn theme_invalid_shows_error() {
    let mut state = fresh_state();
    state.config.theme_name = "dracula".to_string();
    exec(&mut state, "/theme not-a-real-theme");

    // Invalid themes should NOT be set
    assert_eq!(state.config.theme_name, "dracula", "invalid theme should not be set");
    let sys_msgs: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    assert_eq!(sys_msgs.len(), 1);
    assert!(
        sys_msgs[0].content().contains("not found"),
        "should warn about invalid theme: {}",
        sys_msgs[0].content()
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
        matches!(dialog, crate::commands::DialogState::Active { kind: DialogKind::Generic, panels: stack } if stack.current().unwrap().id == "theme"),
        "should open theme panel dialog"
    );
}

#[test]
fn theme_persisted_in_session() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let store = tmp_store();
    // FIXME: Audit that the environment access only happens in single-threaded code.
    unsafe { std::env::set_var("RUNIE_SESSIONS_DIR", store.dir().to_path_buf()) };

    let mut state = fresh_state();
    state.config.theme_name = "nord".to_string();
    exec(&mut state, "/save themed"); // Opens form with pre-filled name
    state.update(Event::submit()); // Submits the form

    let jsonl_store = crate::session::store::SessionStore::new(store.dir().to_path_buf());
    let events = jsonl_store.load_events("themed").unwrap();
    let mut loaded = crate::model::AppState::default();
    crate::session::replay::replay_events(&mut loaded, &events);
    assert_eq!(loaded.config.theme_name, "nord");

    // FIXME: Audit that the environment access only happens in single-threaded code.
    unsafe { std::env::remove_var("RUNIE_SESSIONS_DIR") };
}

#[test]
fn theme_selector_switch_updates_state() {
    let mut state = fresh_state();
    palette_select(&mut state, "theme");
    assert!(
        state.open_dialog.is_some(),
        "theme selector dialog should be open"
    );

    // Filter to dracula and submit to apply the theme while keeping dialog open.
    for c in "dracula".chars() {
        state.update(crate::Event::PaletteFilter(c));
    }
    state.update(crate::Event::PaletteSelect);

    assert_eq!(
        state.config.theme_name, "dracula",
        "selecting dracula in theme picker should switch theme"
    );
    assert!(
        state.open_dialog.is_some(),
        "theme picker should stay open for live preview"
    );
}
