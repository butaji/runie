//! Diagnostics tests (Layer 1 + Layer 2)

use crate::event::Event;
use crate::model::AppState;

#[test]
fn diagnostics_emits_event() {
    let mut state = AppState::default();
    let cmd = state.registry.get("diagnostics").unwrap();
    let result = (cmd.handler)(&mut state, "");
    assert!(matches!(result, crate::commands::CommandResult::Event(Event::ShowDiagnostics)));
}

#[test]
fn diagnostics_shows_config_path() {
    let mut state = AppState::default();
    state.update(Event::ShowDiagnostics);
    let last = state.messages.last().unwrap();
    assert!(last.content.contains("Diagnostics:"));
    assert!(last.content.contains("Config:"));
}

#[test]
fn diagnostics_shows_providers() {
    let mut state = AppState::default();
    state.current_provider = "openai".to_string();
    state.current_model = "gpt-4o".to_string();
    state.update(Event::ShowDiagnostics);
    let last = state.messages.last().unwrap();
    assert!(last.content.contains("openai/gpt-4o"), "Should show provider/model: {}", last.content);
}
