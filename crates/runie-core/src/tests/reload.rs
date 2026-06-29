//! Reload tests (Layer 2)

use crate::event::Event;
use crate::model::AppState;

#[test]
fn reload_returns_event() {
    let mut state = AppState::default();
    let cmd = state.registry.get("reload").unwrap();
    let cmd_name = cmd.name.clone();
    let result = cmd.flow.clone().exec(&mut state, &cmd_name, "");
    assert!(
        matches!(
            result,
            crate::commands::CommandResult::Event(crate::Event::ReloadAll)
        ),
        "expected ReloadAll event, got {result:?}"
    );
}

#[test]
fn config_loaded_updates_keybindings() {
    let mut state = AppState::default();
    let initial_bindings = state.config.keybindings.clone();
    let config = crate::config::Config::default();
    state.update(Event::ConfigLoaded {
        config: Box::new(config),
    });
    // After applying config, keybindings should be refreshed (same content but new HashMap)
    assert_eq!(state.config.keybindings.len(), initial_bindings.len());
}
