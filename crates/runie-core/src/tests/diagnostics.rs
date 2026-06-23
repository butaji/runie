//! Diagnostics tests (Layer 1 + Layer 2)

use crate::event::SystemEvent;

use crate::model::AppState;

#[test]
fn diagnostics_emits_event() {
    let mut state = AppState::default();
    let cmd = state.registry.get("diagnostics").unwrap();
    let cmd_name = cmd.name.clone();
    let result = cmd.flow.clone().exec(&mut state, &cmd_name, "");
    assert!(matches!(
        result,
        crate::commands::CommandResult::Event(SystemEvent::ShowDiagnostics)
    ));
}

#[test]
fn diagnostics_shows_config_path() {
    let mut state = AppState::default();
    state.update(SystemEvent::ShowDiagnostics);
    let last = state.session.messages.last().unwrap();
    assert!(last.content().contains("Diagnostics:"));
    assert!(last.content().contains("Config:"));
}

#[test]
fn diagnostics_shows_providers() {
    let mut state = AppState::default();
    state.config.current_provider = "openai".to_string();
    state.config.current_model = "gpt-4o".to_string();
    state.update(SystemEvent::ShowDiagnostics);
    let last = state.session.messages.last().unwrap();
    assert!(
        last.content().contains("openai/gpt-4o"),
        "Should show provider/model: {}",
        last.content()
    );
}
