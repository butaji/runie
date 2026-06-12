//! Reload tests (Layer 2)

use crate::event::Event;
use crate::model::AppState;

#[test]
fn reload_emits_event() {
    let mut state = AppState::default();
    let cmd = state.registry.get("reload").unwrap();
    let cmd_name = cmd.name.clone();
    let result = cmd.flow.clone().exec(&mut state, &cmd_name, "");
    assert!(matches!(result, crate::commands::CommandResult::Event(Event::ReloadAll)));
}

#[test]
fn reload_updates_keybindings() {
    let mut state = AppState::default();
    let initial_bindings = state.config.keybindings.clone();
    state.update(Event::ReloadAll);
    // After reload, keybindings should be refreshed (same content but new HashMap)
    assert_eq!(state.config.keybindings.len(), initial_bindings.len());
    assert!(state.session.messages.iter().any(|m| {
        m.role == crate::model::Role::System && m.content.contains("Reloaded")
    }));
}
