use super::{get_history_nav_mode, HistoryNavMode};
use crate::model::AppState;
use crate::Event;

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

    // History navigation requires a non-empty input: on an empty input
    // Up/Down scroll the feed (mouse wheels arrive as arrow keys in
    // terminals with alternate scroll, and must scroll the feed).
    state.update(crate::Event::Input('x'));
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

    // Navigate back (non-empty input opts into history navigation)
    state.update(crate::Event::Input('x'));
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

/// Up on an empty input scrolls the feed up instead of recalling history.
///
/// Terminals with "alternate scroll" (iTerm2, kitty, WezTerm) translate
/// mouse-wheel ticks into ↑/↓ key presses when the app does not capture the
/// mouse. Runie keeps native text selection by not capturing the mouse, so
/// those wheel events are byte-identical to arrow keys — when the input box
/// is empty they must scroll the feed, not cycle prompt history.
#[test]
fn up_on_empty_input_scrolls_feed() {
    let mut state = AppState::default();
    state.push_to_input_history("some command");
    state.submit_user_message("hello".to_string());
    assert_eq!(state.view().scroll, 0);

    state.update(crate::Event::HistoryPrev);

    assert_eq!(state.view().scroll, 1, "Up on empty input must scroll up");
    assert!(
        state.input.input.is_empty(),
        "Up on empty input must not recall history, got {:?}",
        state.input.input
    );
}

/// Down on an empty input scrolls the feed toward newer content.
#[test]
fn down_on_empty_input_scrolls_feed_down() {
    let mut state = AppState::default();
    state.push_to_input_history("some command");
    state.submit_user_message("hello".to_string());
    state.view_mut().scroll = 3;

    state.update(crate::Event::HistoryNext);

    assert_eq!(
        state.view().scroll, 2,
        "Down on empty input must scroll down"
    );
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
    state.input.input = "partial".to_string();

    let mode = get_history_nav_mode(&mut state);
    assert!(matches!(mode, HistoryNavMode::History));
}

/// Empty single-line input selects scroll mode: Up/Down then scroll the
/// feed (this is also what mouse wheels send in alternate-scroll terminals).
#[test]
fn history_nav_mode_selects_scroll_when_input_empty() {
    let mut state = AppState::default();
    assert!(state.input.input.is_empty());

    let mode = get_history_nav_mode(&mut state);
    assert!(matches!(mode, HistoryNavMode::Scroll));
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

// ============================================================================
// Layer 1 — State/Logic: paste sanitization
// ============================================================================

/// Pasted multi-line text keeps its line breaks: the input box is multi-line
/// and grok parity requires preserving newlines (CRLF/CR normalize to LF).
#[test]
fn paste_preserves_newlines() {
    let mut state = AppState::default();
    state.update(Event::Paste("the problem\nthe answer".to_string()));
    assert_eq!(state.input.input, "the problem\nthe answer");

    let mut state = AppState::default();
    state.update(Event::Paste("line one\r\nline two\rline three".to_string()));
    assert_eq!(state.input.input, "line one\nline two\nline three");
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
