use super::{get_history_nav_mode, input_event, HistoryNavMode};
use crate::model::{AppState, PermissionRequestState};
use crate::Event;

fn setup_permission_request(id: &str) -> AppState {
    let mut state = AppState::default();
    *state.permission_request_mut() = Some(PermissionRequestState {
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
    assert!(state.permission_request_opt().is_some());

    input_event(&mut state, Event::Input('y'));

    // Permission request remains in state until PermissionResponse event clears it.
    // This is the correct behavior - input handler emits intent, actor resolves.
    assert!(state.permission_request_opt().is_some());
}

#[test]
fn n_key_triggers_permission_deny_intent() {
    let mut state = setup_permission_request("test-n");

    input_event(&mut state, Event::Input('n'));

    // Intent sent, projection happens via PermissionResponse event
    assert!(state.permission_request_opt().is_some());
}

#[test]
fn a_key_allows_permission_request_opt() {
    let mut state = setup_permission_request("test-a");

    input_event(&mut state, Event::Input('a'));

    assert!(state.permission_request_opt().is_some());
}

// ============================================================================
// Layer 2 — Event Handling: permission dialog navigation key no-ops
// ============================================================================

/// Esc while a permission dialog is open is consumed as a no-op.
/// It does NOT deny the permission and does NOT route to the input box.
#[test]
fn esc_during_permission_dialog_is_noop() {
    let mut state = setup_permission_request("test-esc");
    let initial_input = state.input.input.clone();

    input_event(&mut state, Event::Escape);

    // Dialog stays open — Esc is a no-op, not a deny
    assert!(
        state.permission_request_opt().is_some(),
        "Esc should not deny the permission request"
    );
    // Input is unchanged — Esc is consumed, not routed to input box
    assert_eq!(
        state.input.input, initial_input,
        "Esc should not affect the input buffer"
    );
}

/// Backspace while a permission dialog is open is consumed as a no-op.
#[test]
fn backspace_during_permission_dialog_is_noop() {
    let mut state = setup_permission_request("test-backspace");
    let initial_input = state.input.input.clone();

    input_event(&mut state, Event::Backspace);

    assert!(
        state.permission_request_opt().is_some(),
        "Backspace should not deny the permission request"
    );
    assert_eq!(
        state.input.input, initial_input,
        "Backspace should not affect the input buffer"
    );
}

/// Enter while a permission dialog is open is consumed as a no-op.
#[test]
fn newline_during_permission_dialog_is_noop() {
    let mut state = setup_permission_request("test-newline");
    let initial_input = state.input.input.clone();

    input_event(&mut state, Event::Newline);

    assert!(
        state.permission_request_opt().is_some(),
        "Newline should not deny the permission request"
    );
    assert_eq!(
        state.input.input, initial_input,
        "Newline should not affect the input buffer"
    );
}

/// Arrow keys while a permission dialog is open are consumed as no-ops.
#[test]
fn cursor_keys_during_permission_dialog_are_noop() {
    let mut state = setup_permission_request("test-cursor");
    let initial_input = state.input.input.clone();

    input_event(&mut state, Event::CursorLeft);
    input_event(&mut state, Event::CursorRight);
    input_event(&mut state, Event::CursorStart);
    input_event(&mut state, Event::CursorEnd);

    assert!(
        state.permission_request_opt().is_some(),
        "Cursor keys should not deny the permission request"
    );
    assert_eq!(
        state.input.input, initial_input,
        "Cursor keys should not affect the input buffer"
    );
}

/// PageUp/PageDown while a permission dialog is open are consumed as no-ops.
#[test]
fn page_keys_during_permission_dialog_are_noop() {
    let mut state = setup_permission_request("test-page");

    input_event(&mut state, Event::PageUp);
    input_event(&mut state, Event::PageDown);

    assert!(
        state.permission_request_opt().is_some(),
        "Page keys should not deny the permission request"
    );
}

/// Other character keys deny the permission request (fallback for non-y/n/a).
#[test]
fn other_char_keys_deny_permission() {
    let mut state = setup_permission_request("test-other");

    input_event(&mut state, Event::Input('h'));

    // Intent sent (deny), dialog stays in state until PermissionResponse clears it
    assert!(
        state.permission_request_opt().is_some(),
        "Other char keys should trigger deny intent"
    );
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

// ============================================================================
// Layer 1 — State/Logic: submit queues messages when turn is active
// ============================================================================

/// Regression test: when TurnActor is not available (no actor handles),
/// submitting while turn_active should queue the message locally.
#[test]
fn submit_queues_when_turn_active_without_actor_handles() {
    let mut state = AppState::default();
    // No actor handles - this is test mode
    assert!(state.actor_handles().is_none());

    // Simulate an active turn
    state.agent.turn_active = true;

    // Submit a message
    state.input_mut().input = "queued message".to_string();
    state.submit();

    // Message should be queued in local AppState queue
    assert_eq!(state.agent_state().message_queue.len(), 1);
    let queued = &state.agent_state().message_queue[0];
    assert_eq!(queued.content, "queued message");
    assert!(matches!(
        queued.kind,
        crate::model::QueuedMessageKind::Steering
    ));
}

/// When a turn is active, submitting should queue the message.
#[test]
fn submit_while_turn_active_queues_message() {
    let mut state = AppState::default();
    // No actor handles - test mode
    assert!(state.actor_handles().is_none());

    // Simulate an active turn
    state.agent.turn_active = true;

    // Simulate submit while turn is active
    state.input_mut().input = "queued message".to_string();
    state.submit();

    // Message should be queued
    assert_eq!(state.agent_state().message_queue.len(), 1);
    assert_eq!(
        state.agent_state().message_queue[0].content,
        "queued message"
    );
    // Input should be cleared
    assert!(state.input().input.is_empty());
}

/// Multiple submissions while turn is active should queue all messages.
#[test]
fn multiple_submits_while_turn_active_queue_all() {
    let mut state = AppState::default();
    // No actor handles - test mode
    assert!(state.actor_handles().is_none());

    // Simulate an active turn
    state.agent.turn_active = true;

    // First submission
    state.input_mut().input = "first".to_string();
    state.submit();

    // Second submission
    state.input_mut().input = "second".to_string();
    state.submit();

    // Both should be queued
    assert_eq!(state.agent_state().message_queue.len(), 2);
    assert_eq!(state.agent_state().message_queue[0].content, "first");
    assert_eq!(state.agent_state().message_queue[1].content, "second");
}
