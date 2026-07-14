use super::{get_history_nav_mode, HistoryNavMode};
use crate::model::AppState;
use crate::Event;

// ============================================================================
// Layer 2 — Event Handling: history recall from an EMPTY input (grok parity)
// ============================================================================

/// Up on an empty input recalls the latest history entry (readline/grok
/// behavior). The feed is scrolled with PgUp/PgDn and Esc nav mode instead.
#[test]
fn history_prev_on_empty_input_recalls_latest() {
    let mut state = AppState::default();
    state.update(crate::Event::Input('a'));
    state.update(Event::submit());
    state.update(crate::Event::Input('b'));
    state.update(Event::submit());
    assert!(state.input.input.is_empty());

    state.update(crate::Event::HistoryPrev);
    assert_eq!(state.input.input, "b");

    state.update(crate::Event::HistoryPrev);
    assert_eq!(state.input.input, "a");
}

/// Down past the newest entry returns to the empty draft.
#[test]
fn history_next_past_newest_clears_to_empty() {
    let mut state = AppState::default();
    state.update(crate::Event::Input('a'));
    state.update(Event::submit());
    state.update(crate::Event::Input('b'));
    state.update(Event::submit());

    state.update(crate::Event::HistoryPrev);
    assert_eq!(state.input.input, "b");
    state.update(crate::Event::HistoryPrev);
    assert_eq!(state.input.input, "a");

    state.update(crate::Event::HistoryNext);
    assert_eq!(state.input.input, "b");
    state.update(crate::Event::HistoryNext);
    assert!(state.input.input.is_empty());
}

/// Up at the oldest entry stays there (no wrap-around) and flashes.
#[test]
fn history_prev_at_oldest_stays_and_flashes() {
    let mut state = AppState::default();
    state.update(crate::Event::Input('a'));
    state.update(Event::submit());

    state.update(crate::Event::HistoryPrev);
    assert_eq!(state.input.input, "a");
    state.input.input_flash = 0;

    state.update(crate::Event::HistoryPrev);
    assert_eq!(
        state.input.input, "a",
        "must not wrap past the oldest entry"
    );
    assert!(state.input.input_flash > 0, "oldest boundary should flash");
}

/// Down on an empty input (nothing newer) is a no-op flash.
#[test]
fn history_next_on_empty_input_flashes() {
    let mut state = AppState::default();
    state.update(crate::Event::Input('a'));
    state.update(Event::submit());
    assert_eq!(state.view().scroll, 0);

    state.update(crate::Event::HistoryNext);

    assert!(state.input.input.is_empty());
    assert_eq!(
        state.view().scroll,
        0,
        "Down on empty input must not scroll the feed"
    );
    assert!(state.input.input_flash > 0);
}

/// Editing a recalled entry leaves history mode: further Up moves the
/// cursor instead of replacing the edit with the next-older entry.
#[test]
fn editing_recalled_entry_exits_history_mode() {
    let mut state = AppState::default();
    state.update(crate::Event::Input('a'));
    state.update(Event::submit());
    state.update(crate::Event::Input('b'));
    state.update(Event::submit());

    state.update(crate::Event::HistoryPrev);
    assert_eq!(state.input.input, "b");

    // Edit the recalled entry.
    state.update(crate::Event::Input('!'));
    assert_eq!(state.input.input, "b!");

    // Up must NOT clobber the edit with the older entry: grok never disturbs
    // a draft; with text in the box Up moves the cursor to the start.
    state.update(crate::Event::HistoryPrev);
    assert_eq!(state.input.input, "b!", "edit must survive Up");
    assert_eq!(
        state.input.cursor_pos, 0,
        "Up with text moves cursor to start"
    );
}

// ============================================================================
// Layer 2 — Event Handling: arrows with text move the cursor (grok parity)
// ============================================================================

/// Up on a single-line draft moves the cursor to the start of the text and
/// never touches history — drafts cannot be clobbered by arrow keys.
#[test]
fn single_line_up_moves_cursor_to_start() {
    let mut state = AppState::default();
    state.update(crate::Event::Input('a'));
    state.update(Event::submit()); // seed history
    for c in "draft".chars() {
        state.update(crate::Event::Input(c));
    }
    assert_eq!(state.input.cursor_pos, 5);

    state.update(crate::Event::HistoryPrev);

    assert_eq!(state.input.input, "draft", "draft must not be replaced");
    assert_eq!(state.input.cursor_pos, 0);
}

/// Down on a single-line draft moves the cursor to the end of the text.
#[test]
fn single_line_down_moves_cursor_to_end() {
    let mut state = AppState::default();
    for c in "draft".chars() {
        state.update(crate::Event::Input(c));
    }
    state.update(crate::Event::CursorStart);
    assert_eq!(state.input.cursor_pos, 0);

    state.update(crate::Event::HistoryNext);

    assert_eq!(state.input.input, "draft");
    assert_eq!(state.input.cursor_pos, 5);
}

/// Up on the first line of a multi-line draft moves the cursor to the very
/// start of the input (grok), it does not flash or recall history.
#[test]
fn multiline_up_on_first_line_goes_to_input_start() {
    let mut state = AppState::default();
    state.update(crate::Event::Input('a'));
    state.update(Event::submit()); // seed history
    for c in "line1".chars() {
        state.update(crate::Event::Input(c));
    }
    state.update(crate::Event::Newline);
    for c in "line2".chars() {
        state.update(crate::Event::Input(c));
    }
    // Cursor to the first line, column 3.
    state.update(crate::Event::CursorStart);
    state.update(crate::Event::CursorRight);
    state.update(crate::Event::CursorRight);
    state.update(crate::Event::CursorRight);
    assert_eq!(state.input.cursor_pos, 3);
    state.input.input_flash = 0;

    state.update(crate::Event::HistoryPrev);

    assert_eq!(state.input.input, "line1\nline2");
    assert_eq!(
        state.input.cursor_pos, 0,
        "Up on first line → start of input"
    );
    assert_eq!(state.input.input_flash, 0, "no flash on first-line Up");
}

/// Down on the last line of a multi-line draft moves the cursor to the very
/// end of the input (grok), it does not flash.
#[test]
fn multiline_down_on_last_line_goes_to_input_end() {
    let mut state = AppState::default();
    for c in "line1".chars() {
        state.update(crate::Event::Input(c));
    }
    state.update(crate::Event::Newline);
    for c in "line2".chars() {
        state.update(crate::Event::Input(c));
    }
    // Cursor to the last line, column 2.
    state.input.cursor_pos = 8; // "line1\n" + 2
    assert_eq!(state.input.cursor_pos, 8);
    state.input.input_flash = 0;

    state.update(crate::Event::HistoryNext);

    assert_eq!(state.input.input, "line1\nline2");
    assert_eq!(
        state.input.cursor_pos,
        "line1\nline2".len(),
        "Down on last line → end of input"
    );
    assert_eq!(state.input.input_flash, 0, "no flash on last-line Down");
}

/// Up preserves the cursor column across lines, clamped to the target line's
/// length — including for the third line and beyond.
/// Regression: the column cap used the previous line's start offset instead
/// of its length, so moving from line 3 to line 2 landed too far right.
#[test]
fn multiline_up_preserves_column_from_third_line() {
    let mut state = AppState::default();
    // "ab\nxyz\nw" — moving Up from line 3 col 1 must land on line 2 col 1.
    for c in "ab".chars() {
        state.update(crate::Event::Input(c));
    }
    state.update(crate::Event::Newline);
    for c in "xyz".chars() {
        state.update(crate::Event::Input(c));
    }
    state.update(crate::Event::Newline);
    state.update(crate::Event::Input('w'));
    // cursor at end: "ab\nxyz\nw".len() == 8; line 3 starts at 7, col 1.
    assert_eq!(state.input.cursor_pos, 8);

    state.update(crate::Event::HistoryPrev);

    // Line 2 starts at byte 3 ("ab\n"); col 1 → pos 4 (on 'y').
    assert_eq!(state.input.cursor_pos, 4, "column must be preserved");
}

/// A recalled multi-line history entry keeps navigating history (Up recalls
/// the next-older entry) instead of moving the cursor within the text.
#[test]
fn recalled_multiline_entry_keeps_history_navigation() {
    let mut state = AppState::default();
    for c in "one".chars() {
        state.update(crate::Event::Input(c));
    }
    state.update(Event::submit());
    for c in "two".chars() {
        state.update(crate::Event::Input(c));
    }
    state.update(crate::Event::Newline);
    for c in "lines".chars() {
        state.update(crate::Event::Input(c));
    }
    state.update(Event::submit());

    // Recall the multi-line entry, then Up again must recall "one".
    state.update(crate::Event::HistoryPrev);
    assert_eq!(state.input.input, "two\nlines");
    state.update(crate::Event::HistoryPrev);
    assert_eq!(state.input.input, "one");
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

    let mode = get_history_nav_mode(&mut state);
    assert!(matches!(mode, HistoryNavMode::PathComplete));
}

#[test]
fn history_nav_mode_selects_multiline_when_multiline_input() {
    let mut state = AppState::default();
    state.input.input = "line1\nline2".to_string();

    let mode = get_history_nav_mode(&mut state);
    assert!(matches!(mode, HistoryNavMode::MultiLine));
}

/// A non-empty single-line draft selects cursor mode: Up/Down move the
/// cursor to the start/end of the text (grok parity), never history.
#[test]
fn history_nav_mode_selects_single_line_when_draft_present() {
    let mut state = AppState::default();
    state.input.input = "partial".to_string();

    let mode = get_history_nav_mode(&mut state);
    assert!(matches!(mode, HistoryNavMode::SingleLine));
}

/// Empty input selects history mode: Up recalls the latest entry.
#[test]
fn history_nav_mode_selects_history_when_input_empty() {
    let mut state = AppState::default();
    assert!(state.input.input.is_empty());

    let mode = get_history_nav_mode(&mut state);
    assert!(matches!(mode, HistoryNavMode::History));
}

/// An unmodified recalled entry selects recall mode regardless of content
/// (including multi-line entries): arrows keep navigating history.
#[test]
fn history_nav_mode_selects_recall_while_recalled_entry_unmodified() {
    let mut state = AppState::default();
    state.input.input_history = vec!["old".to_string(), "two\nlines".to_string()];
    state.update(crate::Event::HistoryPrev);
    assert_eq!(state.input.input, "two\nlines");

    let mode = get_history_nav_mode(&mut state);
    assert!(matches!(mode, HistoryNavMode::Recall));
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
