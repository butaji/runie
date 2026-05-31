//! Input submission tests.
//!
//! Tests for message submission behavior including:
//! - Normal submission (clears textarea, spawns agent)
//! - Empty/whitespace-only rejection
//! - Long message handling
//! - Submit while agent running (cancel + proceed)
//! - Submit with no model (error)
//! - Submit during onboarding (deferred)

use crate::tui::state::{AppState, CommandPaletteState, Msg, Cmd, TuiMode, TopBarState};
use crate::components::{CommandPalette, MessageItem};
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
        thinking_start: None,
        thinking_duration: None,
        is_thinking: false,
        current_thinking_text: String::new(),
        mock_mode: false,
        top_bar: TopBarState::default(),
    }
}

fn make_state_with_model(model: &str) -> AppState {
    let mut state = make_state();
    state.current_model = Some(model.to_string());
    state
}

fn make_state_with_text(text: &str) -> AppState {
    let mut state = make_state_with_model("gpt-4");
    state.textarea = TextArea::new(vec![text.to_string()]);
    state
}

fn type_char(state: &mut AppState, c: char) {
    state.textarea.input(Input { key: Key::Char(c), ctrl: false, alt: false, shift: false });
}

// ─── Normal Submission ─────────────────────────────────────────────────────────

#[test]
fn test_submit_normal_clears_textarea() {
    let mut state = make_state_with_text("Hello, world!");
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Submit);

    assert!(state.textarea.is_empty(), "Textarea should be cleared after submit");
}

#[test]
fn test_submit_normal_spawns_agent() {
    let mut state = make_state_with_text("Hello, world!");
    let mut palette = CommandPalette::new();

    let cmds = update(&mut state, &mut palette, Msg::Submit);

    assert!(state.agent_running, "Agent should be running after submit");
    assert_eq!(cmds.len(), 1);
    assert!(matches!(&cmds[0], Cmd::SpawnAgent { .. }));
}

#[test]
fn test_submit_normal_adds_user_message() {
    let mut state = make_state_with_text("Hello, world!");
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Submit);

    assert_eq!(state.messages.len(), 2); // user + placeholder
    assert!(matches!(&state.messages[0], MessageItem::User { text, .. } if text == "Hello, world!"));
}

#[test]
fn test_submit_normal_adds_placeholder_assistant() {
    let mut state = make_state_with_text("Hello, world!");
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Submit);

    assert!(matches!(&state.messages[1], MessageItem::Assistant { text, .. } if text.is_empty()));
}

// ─── Empty Message Rejection ──────────────────────────────────────────────────

#[test]
fn test_submit_empty_rejected() {
    let mut state = make_state_with_model("gpt-4");
    let mut palette = CommandPalette::new();

    let cmds = update(&mut state, &mut palette, Msg::Submit);

    assert!(cmds.is_empty(), "No commands should be issued for empty submit");
    assert!(!state.agent_running, "Agent should not be running");
    assert_eq!(state.messages.len(), 0, "No messages should be added");
}

#[test]
fn test_submit_empty_shows_hint() {
    let mut state = make_state_with_model("gpt-4");
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Submit);

    assert!(state.input_right_info.contains("Type a message"));
}

// ─── Whitespace-Only Rejection ────────────────────────────────────────────────

#[test]
fn test_submit_whitespace_only_rejected() {
    let mut state = make_state_with_model("gpt-4");
    state.textarea = TextArea::new(vec!["   ".to_string()]);
    let mut palette = CommandPalette::new();

    let cmds = update(&mut state, &mut palette, Msg::Submit);

    assert!(cmds.is_empty(), "Whitespace-only should be rejected");
    assert!(!state.agent_running);
}

#[test]
fn test_submit_tabs_only_rejected() {
    let mut state = make_state_with_model("gpt-4");
    state.textarea = TextArea::new(vec!["\t\t".to_string()]);
    let mut palette = CommandPalette::new();

    let cmds = update(&mut state, &mut palette, Msg::Submit);

    assert!(cmds.is_empty());
}

#[test]
fn test_submit_newlines_only_rejected() {
    let mut state = make_state_with_model("gpt-4");
    state.textarea = TextArea::new(vec!["\n\n\n".to_string()]);
    let mut palette = CommandPalette::new();

    let cmds = update(&mut state, &mut palette, Msg::Submit);

    assert!(cmds.is_empty());
}

// ─── Long Message Handling ─────────────────────────────────────────────────────

#[test]
fn test_submit_1000_chars_succeeds() {
    let text = "a".repeat(1000);
    let mut state = make_state_with_text(&text);
    let mut palette = CommandPalette::new();

    let cmds = update(&mut state, &mut palette, Msg::Submit);

    assert!(state.agent_running);
    assert!(!cmds.is_empty());
    if let MessageItem::User { text: user_text, .. } = &state.messages[0] {
        assert_eq!(user_text.len(), 1000);
    } else {
        panic!("Expected User message");
    }
}

#[test]
fn test_submit_10000_chars_succeeds() {
    let text = "x".repeat(10000);
    let mut state = make_state_with_text(&text);
    let mut palette = CommandPalette::new();

    let cmds = update(&mut state, &mut palette, Msg::Submit);

    assert!(state.agent_running);
    assert!(!cmds.is_empty());
    if let MessageItem::User { text: user_text, .. } = &state.messages[0] {
        assert_eq!(user_text.len(), 10000);
    } else {
        panic!("Expected User message");
    }
}

// ─── Submit While Agent Running ────────────────────────────────────────────────

#[test]
fn test_submit_while_agent_running_cancels_old() {
    let mut state = make_state_with_text("First");
    let mut palette = CommandPalette::new();

    // First submit
    update(&mut state, &mut palette, Msg::Submit);
    assert!(state.agent_running);
    let first_messages_count = state.messages.len();

    // Submit again while running
    state.textarea = TextArea::new(vec!["Second".to_string()]);
    update(&mut state, &mut palette, Msg::Submit);

    // Old placeholder should be removed, new user added
    assert!(state.messages.len() >= first_messages_count);
    assert!(state.agent_running);
}

#[test]
fn test_submit_while_agent_running_proceeds() {
    let mut state = make_state_with_text("First");
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Submit);
    assert!(state.agent_running);

    state.textarea = TextArea::new(vec!["Second".to_string()]);
    let cmds = update(&mut state, &mut palette, Msg::Submit);

    assert!(!cmds.is_empty(), "Should have commands for new submit");
}

#[test]
fn test_submit_while_agent_running_removes_old_placeholder() {
    let mut state = make_state_with_text("First");
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Submit);
    let placeholder_count = state.messages.iter().filter(|m| matches!(m, MessageItem::Assistant { text, .. } if text.is_empty())).count();
    assert_eq!(placeholder_count, 1, "Should have one empty placeholder");

    state.textarea = TextArea::new(vec!["Second".to_string()]);
    update(&mut state, &mut palette, Msg::Submit);

    // Old placeholder should be gone
    let new_placeholder_count = state.messages.iter().filter(|m| matches!(m, MessageItem::Assistant { text, .. } if text.is_empty())).count();
    assert_eq!(new_placeholder_count, 1, "Should still have exactly one empty placeholder");
}

// ─── No Model Submit ──────────────────────────────────────────────────────────

#[test]
fn test_submit_no_model_shows_error() {
    let mut state = make_state();
    state.textarea = TextArea::new(vec!["Hello".to_string()]);
    let mut palette = CommandPalette::new();

    let cmds = update(&mut state, &mut palette, Msg::Submit);

    assert!(cmds.is_empty(), "No commands when no model");
    assert!(!state.agent_running);
    assert!(state.messages.iter().any(|m| matches!(m, MessageItem::Error { .. })));
}

#[test]
fn test_submit_no_model_adds_user_message() {
    let mut state = make_state();
    state.textarea = TextArea::new(vec!["Hello".to_string()]);
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Submit);

    assert!(matches!(&state.messages[0], MessageItem::User { text, .. } if text == "Hello"));
}

#[test]
fn test_submit_empty_model_shows_error() {
    let mut state = make_state();
    state.current_model = Some("".to_string());
    state.textarea = TextArea::new(vec!["Hello".to_string()]);
    let mut palette = CommandPalette::new();

    let cmds = update(&mut state, &mut palette, Msg::Submit);

    assert!(cmds.is_empty());
    assert!(state.messages.iter().any(|m| matches!(m, MessageItem::Error { .. })));
}

// ─── Multi-line Submit ─────────────────────────────────────────────────────────

#[test]
fn test_submit_multiline_text() {
    let mut state = make_state_with_model("gpt-4");
    state.textarea = TextArea::new(vec!["line1".to_string(), "line2".to_string()]);
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Submit);

    assert!(state.agent_running);
    if let MessageItem::User { text, .. } = &state.messages[0] {
        assert_eq!(text, "line1\nline2");
    } else {
        panic!("Expected User message");
    }
}

#[test]
fn test_submit_enter_creates_multiline() {
    let mut state = make_state_with_model("gpt-4");
    let mut palette = CommandPalette::new();

    type_char(&mut state, 'h');
    type_char(&mut state, 'i');
    state.textarea.insert_newline();
    type_char(&mut state, 't');
    type_char(&mut state, 'h');
    type_char(&mut state, 'e');
    type_char(&mut state, 'r');
    type_char(&mut state, 'e');

    update(&mut state, &mut palette, Msg::Submit);

    assert!(state.agent_running);
    if let MessageItem::User { text, .. } = &state.messages[0] {
        assert_eq!(text, "hi\nthere");
    } else {
        panic!("Expected User message");
    }
}

// ─── Input History on Submit ──────────────────────────────────────────────────

#[test]
fn test_submit_saves_to_history() {
    let mut state = make_state_with_text("Hello");
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Submit);

    assert_eq!(state.input_history.len(), 1);
    assert_eq!(state.input_history[0], "Hello");
}

#[test]
fn test_submit_clears_history_index() {
    let mut state = make_state_with_text("First");
    let mut palette = CommandPalette::new();

    // Submit first message
    update(&mut state, &mut palette, Msg::Submit);

    // Add another and submit
    state.textarea = TextArea::new(vec!["Second".to_string()]);
    update(&mut state, &mut palette, Msg::Submit);

    assert!(state.input_history_index.is_none());
}
