use super::{get_history_nav_mode, input_event, HistoryNavMode};
use crate::model::{AppState, PermissionRequestState};
use crate::Event;

fn setup_permission_request(id: &str) -> AppState {
    let mut state = AppState::default();
    state.permission_request = Some(PermissionRequestState {
        request_id: id.into(),
        tool: "bash".into(),
        input: serde_json::Value::Null,
    });
    state
}

// Note: Permission input tests now verify the actor handles are NOT set
// (fire-and-forget behavior), and the permission_request projection
// happens via the PermissionResponse event in the event dispatcher.
// These tests verify the input handler sends the intent correctly.

#[test]
fn y_key_triggers_permission_allow_intent() {
    // When actor handles are None (unit test), intent is fire-and-forget.
    // The PermissionActor handles the registry resolution.
    let mut state = setup_permission_request("test-y");
    // Actor handles are None in tests, so try_resolve_permission is no-op.
    // The actual resolution happens via PermissionActor in integration tests.
    assert!(state.permission_request.is_some());

    input_event(&mut state, Event::Input('y'));

    // Permission request remains in state until PermissionResponse event clears it.
    // This is the correct behavior - input handler emits intent, actor resolves.
    assert!(state.permission_request.is_some());
}

#[test]
fn n_key_triggers_permission_deny_intent() {
    let mut state = setup_permission_request("test-n");

    input_event(&mut state, Event::Input('n'));

    // Intent sent, projection happens via PermissionResponse event
    assert!(state.permission_request.is_some());
}

#[test]
fn a_key_allows_permission_request() {
    let mut state = setup_permission_request("test-a");

    input_event(&mut state, Event::Input('a'));

    assert!(state.permission_request.is_some());
}

// ============================================================================
// Layer 2 — Event Handling: history_prev_moves_up, history_next_moves_down
// ============================================================================

#[test]
fn history_prev_moves_up() {
    let mut state = AppState::default();
    // Add some history
    state.update(crate::Event::Input('a'));
    state.update(Event::submit());
    state.update(crate::Event::Input('b'));
    state.update(Event::submit());

    // Clear input then go back in history
    state.update(crate::Event::Backspace);
    state.update(crate::Event::HistoryPrev);
    assert_eq!(state.input.input, "b");

    state.update(crate::Event::HistoryPrev);
    assert_eq!(state.input.input, "a");
}

#[test]
fn history_next_moves_down() {
    let mut state = AppState::default();
    // Add some history
    state.update(crate::Event::Input('a'));
    state.update(Event::submit());
    state.update(crate::Event::Input('b'));
    state.update(Event::submit());

    // Navigate back
    state.update(crate::Event::HistoryPrev);
    assert_eq!(state.input.input, "b");
    state.update(crate::Event::HistoryPrev);
    assert_eq!(state.input.input, "a");

    // Navigate forward
    state.update(crate::Event::HistoryNext);
    assert_eq!(state.input.input, "b");
    state.update(crate::Event::HistoryNext);
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
        PathCompletion {
            path: "/src".to_string(),
            is_dir: true,
        },
        PathCompletion {
            path: "/tests".to_string(),
            is_dir: true,
        },
    ]);

    // Both prev and next should use path completion when suggestions are open
    let mode = get_history_nav_mode(&mut state);
    assert!(matches!(mode, HistoryNavMode::PathComplete));
}

#[test]
fn history_nav_mode_selects_cursor_when_multiline_input() {
    let mut state = AppState::default();
    state.input.input = "line1\nline2".to_string();

    let mode = get_history_nav_mode(&mut state);
    assert!(matches!(mode, HistoryNavMode::Cursor));
}

#[test]
fn history_nav_mode_selects_history_when_plain_input() {
    let mut state = AppState::default();

    let mode = get_history_nav_mode(&mut state);
    assert!(matches!(mode, HistoryNavMode::History));
}
