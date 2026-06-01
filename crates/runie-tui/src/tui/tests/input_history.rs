//! Input history navigation tests.
//!
//! Tests for history up/down navigation including:
//! - Basic up/down navigation
//! - History at boundaries (oldest/newest)
//! - Draft restore on navigation
//! - History limit (100 entries)
//! - Empty history handling

use crate::tui::state::{AppState, CommandPaletteState, Msg, TuiMode, TopBarState};
use crate::components::CommandPalette;
use crate::tui::update::update;
use ratatui_textarea::{TextArea, Input, Key};
use runie_ai::TokenUsage as AiTokenUsage;

fn make_state() -> AppState {
    AppState {
        messages: vec![],
        textarea: TextArea::default(),
        input_right_info: String::new(),
        mode: TuiMode::Chat,
        running: true,
        show_sidebar: false,
        agent_running: false,
        current_model: None,
        context: Default::default(),
        permission_modal: Default::default(),
        command_palette: CommandPaletteState::default(),
        scroll: Default::default(),
        animation: Default::default(),
        diff_viewer: None,
        token_usage: AiTokenUsage::default(),
        session_token_usage: AiTokenUsage::default(),
        session_tree: Default::default(),
        background_jobs: Vec::new(),
        onboarding: None,
        terminal_size: (0, 0),
        clear_input_confirm: Default::default(),
        model_picker: None,
        agent_start_time: None,
        input_history: Vec::new(),
        input_history_index: None,
        input_draft: String::new(),
        status_header: None,
        status_details: None,
        status_start_time: None,
        thinking: None,
        mock_mode: false,
        top_bar: TopBarState::default(),
        show_thoughts: false,
    }
}

fn make_state_with_history(history: Vec<&str>) -> AppState {
    let mut state = make_state();
    state.input_history = history.into_iter().map(String::from).collect();
    state
}

fn make_state_with_model(model: &str) -> AppState {
    let mut state = make_state();
    state.current_model = Some(model.to_string());
    state
}

fn type_char(state: &mut AppState, c: char) {
    state.textarea.input(Input { key: Key::Char(c), ctrl: false, alt: false, shift: false });
}

// ─── Basic History Navigation ─────────────────────────────────────────────────

#[test]
fn test_history_up_navigates_to_newest() {
    let mut state = make_state_with_history(vec!["first", "second", "third"]);
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::HistoryUp);

    assert_eq!(state.input_history_index, Some(2));
    let text = state.textarea.lines().join("\n");
    assert_eq!(text, "third");
}

#[test]
fn test_history_up_twice_navigates_backward() {
    let mut state = make_state_with_history(vec!["first", "second", "third"]);
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::HistoryUp);
    update(&mut state, &mut palette, Msg::HistoryUp);

    assert_eq!(state.input_history_index, Some(1));
    let text = state.textarea.lines().join("\n");
    assert_eq!(text, "second");
}

#[test]
fn test_history_up_to_oldest() {
    let mut state = make_state_with_history(vec!["first", "second", "third"]);
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::HistoryUp);
    update(&mut state, &mut palette, Msg::HistoryUp);
    update(&mut state, &mut palette, Msg::HistoryUp);

    assert_eq!(state.input_history_index, Some(0));
    let text = state.textarea.lines().join("\n");
    assert_eq!(text, "first");
}

#[test]
fn test_history_down_after_up_navigates_forward() {
    let mut state = make_state_with_history(vec!["first", "second", "third"]);
    let mut palette = CommandPalette::new();

    // Go up to second
    update(&mut state, &mut palette, Msg::HistoryUp);
    update(&mut state, &mut palette, Msg::HistoryUp);
    assert_eq!(state.input_history_index, Some(1));

    // Go down back to third
    update(&mut state, &mut palette, Msg::HistoryDown);

    assert_eq!(state.input_history_index, Some(2));
    let text = state.textarea.lines().join("\n");
    assert_eq!(text, "third");
}

// ─── History Boundaries ───────────────────────────────────────────────────────

#[test]
fn test_history_up_at_oldest_stays_at_oldest() {
    let mut state = make_state_with_history(vec!["first", "second"]);
    let mut palette = CommandPalette::new();

    // Navigate to oldest
    update(&mut state, &mut palette, Msg::HistoryUp);
    update(&mut state, &mut palette, Msg::HistoryUp);
    assert_eq!(state.input_history_index, Some(0));

    // Try to go further up
    update(&mut state, &mut palette, Msg::HistoryUp);

    assert_eq!(state.input_history_index, Some(0), "Should stay at oldest");
}

#[test]
fn test_history_down_at_newest_returns_to_draft() {
    let mut state = make_state_with_history(vec!["first", "second"]);
    state.input_draft = "draft text".to_string();
    let mut palette = CommandPalette::new();

    // Navigate to newest
    update(&mut state, &mut palette, Msg::HistoryUp);
    assert_eq!(state.input_history_index, Some(1));

    // Go down - should return to draft
    update(&mut state, &mut palette, Msg::HistoryDown);

    assert!(state.input_history_index.is_none());
    let text = state.textarea.lines().join("\n");
    assert_eq!(text, "draft text");
}

#[test]
fn test_history_down_at_boundary_no_op() {
    let mut state = make_state_with_history(vec!["first", "second"]);
    state.input_history_index = None;
    let mut palette = CommandPalette::new();

    // Already at draft (index is None), down should be no-op
    update(&mut state, &mut palette, Msg::HistoryDown);

    assert!(state.input_history_index.is_none());
}

// ─── Draft Restore ───────────────────────────────────────────────────────────

#[test]
fn test_history_up_saves_draft() {
    let mut state = make_state_with_history(vec!["first", "second"]);
    type_char(&mut state, 'd');
    type_char(&mut state, 'r');
    type_char(&mut state, 'a');
    type_char(&mut state, 'f');
    type_char(&mut state, 't');
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::HistoryUp);

    assert_eq!(state.input_draft, "draft");
}

#[test]
fn test_history_down_restores_draft() {
    let mut state = make_state_with_history(vec!["first", "second"]);
    state.textarea = TextArea::new(vec!["draft".to_string()]);
    let mut palette = CommandPalette::new();

    // Navigate up first
    update(&mut state, &mut palette, Msg::HistoryUp);
    // Then down
    update(&mut state, &mut palette, Msg::HistoryDown);

    let text = state.textarea.lines().join("\n");
    assert_eq!(text, "draft");
}

#[test]
fn test_history_draft_cleared_after_restore() {
    let mut state = make_state_with_history(vec!["first"]);
    state.input_draft = "draft".to_string();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::HistoryUp);
    update(&mut state, &mut palette, Msg::HistoryDown);

    assert!(state.input_draft.is_empty());
}

// ─── Empty History ─────────────────────────────────────────────────────────────

#[test]
fn test_history_up_empty_does_nothing() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::HistoryUp);

    assert!(state.input_history_index.is_none());
    assert!(state.textarea.lines().join("\n").is_empty());
}

#[test]
fn test_history_down_empty_does_nothing() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::HistoryDown);

    assert!(state.input_history_index.is_none());
}

// ─── History Limit ────────────────────────────────────────────────────────────

#[test]
fn test_history_limit_100_entries() {
    let mut state = make_state_with_model("gpt-4");
    let mut palette = CommandPalette::new();

    // Add 105 messages
    for i in 0..105 {
        state.textarea = TextArea::new(vec![format!("message {}", i)]);
        update(&mut state, &mut palette, Msg::Submit);
        state.agent_running = false; // Reset for next submit
    }

    assert_eq!(state.input_history.len(), 100, "History should be limited to 100");
    assert_eq!(state.input_history[0], "message 5", "Oldest entries should be removed");
    assert_eq!(state.input_history[99], "message 104", "Newest entries should remain");
}

#[test]
fn test_history_at_limit_replaces_oldest() {
    let mut state = make_state_with_model("gpt-4");
    let mut palette = CommandPalette::new();

    // Fill to capacity
    for i in 0..100 {
        state.textarea = TextArea::new(vec![format!("msg{}", i)]);
        update(&mut state, &mut palette, Msg::Submit);
        state.agent_running = false;
    }

    // Add one more
    state.textarea = TextArea::new(vec!["newest".to_string()]);
    update(&mut state, &mut palette, Msg::Submit);

    assert_eq!(state.input_history.len(), 100);
    assert_eq!(state.input_history[0], "msg1", "msg0 should be removed");
    assert_eq!(state.input_history[99], "newest");
}

// ─── Submit Clears History Navigation ─────────────────────────────────────────

#[test]
fn test_submit_clears_history_index() {
    let mut state = make_state_with_history(vec!["first", "second"]);
    let mut palette = CommandPalette::new();

    // Navigate
    update(&mut state, &mut palette, Msg::HistoryUp);
    assert!(state.input_history_index.is_some());

    // Submit
    state.textarea = TextArea::new(vec!["new".to_string()]);
    update(&mut state, &mut palette, Msg::Submit);

    assert!(state.input_history_index.is_none());
}

#[test]
fn test_submit_clears_draft() {
    let mut state = make_state_with_history(vec!["first"]);
    state.input_draft = "draft".to_string();
    let mut palette = CommandPalette::new();

    state.textarea = TextArea::new(vec!["new".to_string()]);
    update(&mut state, &mut palette, Msg::Submit);

    assert!(state.input_draft.is_empty());
}

// ─── Multi-line History Items ─────────────────────────────────────────────────

#[test]
fn test_history_multiline_item() {
    let mut state = make_state();
    state.input_history.push("line1\nline2".to_string());
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::HistoryUp);

    let text = state.textarea.lines().join("\n");
    assert_eq!(text, "line1\nline2");
}

#[test]
fn test_history_navigation_with_multiline() {
    let mut state = make_state();
    state.input_history.push("single".to_string());
    state.input_history.push("multi\nline".to_string());
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::HistoryUp);
    assert_eq!(state.textarea.lines().join("\n"), "multi\nline");

    update(&mut state, &mut palette, Msg::HistoryDown);
    assert_eq!(state.textarea.lines().join("\n"), "single");
}
