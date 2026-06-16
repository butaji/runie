use crate::commands::dsl::handlers::agents::handle_agents;
use crate::event::{DialogEvent, Event};
use crate::model::AppState;

#[test]
fn slash_agents_registered() {
    use crate::commands::CommandRegistry;
    let reg = CommandRegistry::new();
    let cmd = reg.get("agents");
    assert!(cmd.is_some(), "expected /agents command");
}

#[test]
fn slash_agents_handler_returns_open_event() {
    let mut state = AppState::default();
    let result = handle_agents(&mut state, "");
    match result {
        crate::commands::CommandResult::Event(Event::Dialog(DialogEvent::OpenAgentsManager)) => {}
        other => panic!("expected OpenAgentsManager, got {:?}", other),
    }
}

#[test]
fn slash_agents_handler_ignores_args() {
    let mut state = AppState::default();
    let _ = handle_agents(&mut state, "extra args here");
}
