use super::{get_history_nav_mode, input_event, HistoryNavMode};
use crate::event::{Event, InputEvent};
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

// ============================================================================
// Layer 2 — Event Handling: history_prev_moves_up, history_next_moves_down
// ============================================================================

#[test]
fn history_prev_moves_up() {
    let mut state = AppState::default();
    // Add some history
    state.update(InputEvent::Input('a'));
    state.update(Event::submit());
    state.update(InputEvent::Input('b'));
    state.update(Event::submit());

    // Clear input then go back in history
    state.update(InputEvent::Backspace);
    state.update(InputEvent::HistoryPrev);
    assert_eq!(state.input.input, "b");

    state.update(InputEvent::HistoryPrev);
    assert_eq!(state.input.input, "a");
}

#[test]
fn history_next_moves_down() {
    let mut state = AppState::default();
    // Add some history
    state.update(InputEvent::Input('a'));
    state.update(Event::submit());
    state.update(InputEvent::Input('b'));
    state.update(Event::submit());

    // Navigate back
    state.update(InputEvent::HistoryPrev);
    assert_eq!(state.input.input, "b");
    state.update(InputEvent::HistoryPrev);
    assert_eq!(state.input.input, "a");

    // Navigate forward
    state.update(InputEvent::HistoryNext);
    assert_eq!(state.input.input, "b");
    state.update(InputEvent::HistoryNext);
    assert!(state.input.input.is_empty());
}

// ============================================================================
// Layer 1 — State/Logic: history_nav_mode_selects_by_mode
// ============================================================================

#[test]
fn history_nav_mode_selects_path_complete_when_suggestions_open() {
    use crate::path_complete::PathCompletion;

    let mut state = AppState::default();
    state.completion.path_suggestions = Some(vec![
        PathCompletion { path: "/src".to_string(), is_dir: true },
        PathCompletion { path: "/tests".to_string(), is_dir: true },
    ]);

    // Both prev and next should use path completion when suggestions are open
    let mode = get_history_nav_mode(&state);
    assert!(matches!(mode, HistoryNavMode::PathComplete));
}

#[test]
fn history_nav_mode_selects_cursor_when_multiline_input() {
    let mut state = AppState::default();
    state.input.input = "line1\nline2".to_string();

    let mode = get_history_nav_mode(&state);
    assert!(matches!(mode, HistoryNavMode::Cursor));
}

#[test]
fn history_nav_mode_selects_history_when_plain_input() {
    let state = AppState::default();

    let mode = get_history_nav_mode(&state);
    assert!(matches!(mode, HistoryNavMode::History));
}
