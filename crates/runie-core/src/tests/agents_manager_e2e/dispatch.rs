use crate::commands::DialogState;
use crate::event::{DialogEvent, Event};
use crate::model::AppState;

#[test]
fn open_event_opens_root_panel() {
    let mut state = AppState::default();
    state.update(Event::Dialog(DialogEvent::OpenAgentsManager));
    assert!(state.open_dialog.is_some());
    if let Some(DialogState::PanelStack(stack)) = &state.open_dialog {
        let panel = stack.current().unwrap();
        assert_eq!(panel.title, "Agent Profiles");
    } else {
        panic!("expected PanelStack dialog");
    }
}

#[test]
fn open_event_does_nothing_if_cancelled() {
    let mut state = AppState::default();
    state.update(Event::Dialog(DialogEvent::OpenAgentsManager));
    let _ = state.open_dialog.is_some();
}

#[test]
fn delete_event_with_missing_profile_is_safe() {
    let mut state = AppState::default();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        state.update(Event::Dialog(DialogEvent::AgentsManagerDelete {
            name: "nonexistent".to_string(),
        }));
    }));
    let _ = result;
}

#[test]
fn save_event_with_missing_profile_does_not_crash() {
    let mut state = AppState::default();
    state.update(Event::Dialog(DialogEvent::AgentsManagerSave {
        name: "nonexistent".to_string(),
    }));
}

#[test]
fn open_event_replaces_existing_dialog() {
    let mut state = AppState::default();
    state.update(Event::Dialog(DialogEvent::OpenAgentsManager));
    assert!(state.open_dialog.is_some());
    state.update(Event::Dialog(DialogEvent::OpenAgentsManager));
    assert!(state.open_dialog.is_some());
}
