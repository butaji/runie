use super::input_event;
use crate::event::Event;
use crate::model::{AppState, PermissionRequestState};
use crate::permissions::PermissionAction;

fn setup_permission_request(id: &str) -> AppState {
    let mut state = AppState::default();
    state.permission_request = Some(PermissionRequestState {
        request_id: id.into(),
        tool: "bash".into(),
        input: serde_json::Value::Null,
    });
    state
}

#[test]
fn y_allows_pending_permission_request() {
    let mut state = setup_permission_request("test-y");
    let rx = state
        .approval_registry
        .lock()
        .unwrap()
        .register("test-y");

    input_event(&mut state, Event::Input('y'));

    assert!(state.permission_request.is_none());
    assert_eq!(rx.blocking_recv(), Ok(PermissionAction::Allow));
}

#[test]
fn n_denies_pending_permission_request() {
    let mut state = setup_permission_request("test-n");
    let rx = state
        .approval_registry
        .lock()
        .unwrap()
        .register("test-n");

    input_event(&mut state, Event::Input('n'));

    assert!(state.permission_request.is_none());
    assert_eq!(rx.blocking_recv(), Ok(PermissionAction::Deny));
}

#[test]
fn any_other_key_denies_pending_permission_request() {
    let mut state = setup_permission_request("test-other");
    let rx = state
        .approval_registry
        .lock()
        .unwrap()
        .register("test-other");

    input_event(&mut state, Event::Input('x'));

    assert!(state.permission_request.is_none());
    assert_eq!(rx.blocking_recv(), Ok(PermissionAction::Deny));
}
