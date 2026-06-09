//! Reload tests (Layer 2)

use crate::event::Event;
use crate::model::AppState;

#[test]
fn reload_emits_event() {
    let mut state = AppState::default();
    let cmd = state.registry.get("reload").unwrap();
    let result = (cmd.handler)(&mut state, "");
    assert!(matches!(result, crate::commands::CommandResult::Event(Event::ReloadAll)));
}

#[test]
fn reload_updates_keybindings() {
    let mut state = AppState::default();
    let initial_bindings = state.keybindings.clone();
    state.update(Event::ReloadAll);
    // After reload, keybindings should be refreshed (same content but new HashMap)
    assert_eq!(state.keybindings.len(), initial_bindings.len());
    assert!(state.messages.iter().any(|m| {
        m.role == crate::model::Role::System && m.content.contains("Reloaded")
    }));
}
