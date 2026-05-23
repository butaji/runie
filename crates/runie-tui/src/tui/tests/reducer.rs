//! Reducer tests for state updates.

use crate::tui::state::{AppState, AnimationState, CommandPaletteState, Msg, ScrollState, TopBarState, PermissionModalState, TuiMode};
use crate::components::{MessageItem, SessionTreeNavigator};
use crate::tui::update::update;
use runie_agent::{AgentEvent, AgentMessage, ContentPart, PermissionDecision};
use runie_ai::TokenUsage;

fn make_state() -> AppState {
    AppState {
        messages: vec![],
        input_lines: vec![String::new()],
        cursor_col: 0,
        cursor_row: 0,
        input_right_info: String::new(),
        mode: TuiMode::Chat,
        running: true,
        show_sidebar: false,
        agent_running: false,
        current_model: None,
        top_bar: TopBarState::default(),
        permission_modal: PermissionModalState::default(),
        command_palette: CommandPaletteState::default(),
        scroll: ScrollState::default(),
        animation: AnimationState::default(),
        diff_viewer: None,
        token_usage: TokenUsage::default(),
        session_token_usage: TokenUsage::default(),
        session_tree: SessionTreeNavigator::new(),
        background_jobs: Vec::new(),
        onboarding: None,
    }
}

#[test]
fn test_insert_char() {
    let mut state = make_state();
    update(&mut state, Msg::InsertChar('h'));
    update(&mut state, Msg::InsertChar('i'));
    assert_eq!(state.input_lines, vec!["hi"]);
    assert_eq!(state.cursor_col, 2);
}

#[test]
fn test_backspace() {
    let mut state = make_state();
    update(&mut state, Msg::InsertChar('h'));
    update(&mut state, Msg::InsertChar('i'));
    update(&mut state, Msg::Backspace);
    assert_eq!(state.input_lines, vec!["h"]);
    assert_eq!(state.cursor_col, 1);
}

#[test]
fn test_submit_clears_input() {
    let mut state = make_state();
    update(&mut state, Msg::InsertChar('h'));
    update(&mut state, Msg::InsertChar('i'));
    let cmds = update(&mut state, Msg::Submit);
    assert_eq!(state.input_lines, vec![""]);
    assert_eq!(state.messages.len(), 1);
    // Should return a SpawnAgent cmd
    assert_eq!(cmds.len(), 1);
    if let crate::tui::state::Cmd::SpawnAgent { .. } = &cmds[0] {
        // Expected
    } else {
        panic!("Expected SpawnAgent cmd");
    }
    if let MessageItem::User { text, .. } = &state.messages[0] {
        assert_eq!(text, "hi");
    } else {
        panic!("Expected User message");
    }
}

#[test]
fn test_submit_empty_does_nothing() {
    let mut state = make_state();
    let cmds = update(&mut state, Msg::Submit);
    assert_eq!(state.messages.len(), 0);
    assert!(cmds.is_empty());
}

#[test]
fn test_move_cursor() {
    let mut state = make_state();
    update(&mut state, Msg::InsertChar('a'));
    update(&mut state, Msg::InsertChar('b'));
    update(&mut state, Msg::InsertChar('c'));
    assert_eq!(state.cursor_col, 3);

    update(&mut state, Msg::MoveCursorLeft);
    assert_eq!(state.cursor_col, 2);

    update(&mut state, Msg::MoveCursorLeft);
    assert_eq!(state.cursor_col, 1);

    update(&mut state, Msg::MoveCursorRight);
    assert_eq!(state.cursor_col, 2);

    update(&mut state, Msg::MoveCursorToStart);
    assert_eq!(state.cursor_col, 0);

    update(&mut state, Msg::MoveCursorToEnd);
    assert_eq!(state.cursor_col, 3);
}

#[test]
fn test_newline() {
    let mut state = make_state();
    update(&mut state, Msg::InsertChar('h'));
    update(&mut state, Msg::InsertChar('i'));
    update(&mut state, Msg::InsertNewline);
    assert_eq!(state.input_lines, vec!["hi", ""]);
    assert_eq!(state.cursor_row, 1);
    assert_eq!(state.cursor_col, 0);
}

#[test]
fn test_multi_line_submit() {
    let mut state = make_state();
    for c in "line1".chars() {
        update(&mut state, Msg::InsertChar(c));
    }
    update(&mut state, Msg::InsertNewline);
    for c in "line2".chars() {
        update(&mut state, Msg::InsertChar(c));
    }
    update(&mut state, Msg::Submit);

    assert_eq!(state.input_lines, vec![""]);
    assert_eq!(state.messages.len(), 1);
    if let MessageItem::User { text, .. } = &state.messages[0] {
        assert_eq!(text, "line1\nline2");
    } else {
        panic!("Expected User message");
    }
}

#[test]
fn test_quit() {
    let mut state = make_state();
    update(&mut state, Msg::Quit);
    assert!(!state.running);
}

#[test]
fn test_toggle_sidebar() {
    let mut state = make_state();
    assert!(!state.show_sidebar);
    update(&mut state, Msg::ToggleSidebar);
    assert!(state.show_sidebar);
    update(&mut state, Msg::ToggleSidebar);
    assert!(!state.show_sidebar);
}

#[test]
fn test_delete_word_backward() {
    let mut state = make_state();
    // Type "hello world"
    for c in "hello world".chars() {
        update(&mut state, Msg::InsertChar(c));
    }
    assert_eq!(state.cursor_col, 11);

    // Delete word backward → "hello" (removes " world" including space, bash-like)
    update(&mut state, Msg::DeleteWordBackward);
    assert_eq!(state.input_lines[0], "hello");
    assert_eq!(state.cursor_col, 5);

    // Delete word backward → "" (no more words, clears line)
    update(&mut state, Msg::DeleteWordBackward);
    assert_eq!(state.input_lines[0], "");
    assert_eq!(state.cursor_col, 0);
}

#[test]
fn test_delete_to_start() {
    let mut state = make_state();
    for c in "hello".chars() {
        update(&mut state, Msg::InsertChar(c));
    }
    update(&mut state, Msg::MoveCursorToEnd);
    update(&mut state, Msg::DeleteToStart);
    assert_eq!(state.input_lines[0], "");
    assert_eq!(state.cursor_col, 0);
}

#[test]
fn test_delete_forward() {
    let mut state = make_state();
    update(&mut state, Msg::InsertChar('a'));
    update(&mut state, Msg::InsertChar('b'));
    update(&mut state, Msg::InsertChar('c'));
    update(&mut state, Msg::MoveCursorToStart);
    update(&mut state, Msg::DeleteForward);
    assert_eq!(state.input_lines[0], "bc");
}

#[test]
fn test_agent_event_message_start() {
    let mut state = make_state();
    update(
        &mut state,
        Msg::AgentEvent(AgentEvent::MessageStart {
            message: AgentMessage {
                role: "assistant".to_string(),
                content: vec![],
                timestamp: 0,
                usage: None,
                stop_reason: None,
                error_message: None,
            },
        }),
    );
    assert!(state.agent_running);
    assert_eq!(state.messages.len(), 1);
}

#[test]
fn test_agent_event_message_update() {
    let mut state = make_state();
    // Start message
    update(
        &mut state,
        Msg::AgentEvent(AgentEvent::MessageStart {
            message: AgentMessage {
                role: "assistant".to_string(),
                content: vec![],
                timestamp: 0,
                usage: None,
                stop_reason: None,
                error_message: None,
            },
        }),
    );

    // Update with text
    update(
        &mut state,
        Msg::AgentEvent(AgentEvent::MessageUpdate {
            message: AgentMessage {
                role: "assistant".to_string(),
                content: vec![ContentPart::Text {
                    text: "Hello".to_string(),
                }],
                timestamp: 0,
                usage: None,
                stop_reason: None,
                error_message: None,
            },
        }),
    );

    assert_eq!(state.messages.len(), 1);
    if let MessageItem::Assistant { text, .. } = &state.messages[0] {
        assert_eq!(text, "Hello");
    } else {
        panic!("Expected Assistant message");
    }
}

#[test]
fn test_permission_cmds() {
    use crate::tui::state::Cmd;

    let mut state = make_state();

    // PermissionConfirm should return Allow decision
    let cmds = update(&mut state, Msg::PermissionConfirm);
    assert_eq!(cmds.len(), 1);
    if let Cmd::SendPermission { decision } = &cmds[0] {
        assert!(matches!(*decision, PermissionDecision::Allow { .. }));
    } else {
        panic!("Expected SendPermission cmd");
    }

    // PermissionCancel should return Deny decision
    let cmds = update(&mut state, Msg::PermissionCancel);
    if let Cmd::SendPermission { decision } = &cmds[0] {
        assert!(matches!(*decision, PermissionDecision::Deny { .. }));
    }

    // PermissionAlways should return AllowAlways decision
    let cmds = update(&mut state, Msg::PermissionAlways);
    if let Cmd::SendPermission { decision } = &cmds[0] {
        assert!(matches!(*decision, PermissionDecision::AllowAlways { .. }));
    }

    // PermissionSkip should return Skip decision
    let cmds = update(&mut state, Msg::PermissionSkip);
    if let Cmd::SendPermission { decision } = &cmds[0] {
        assert!(matches!(*decision, PermissionDecision::Skip { .. }));
    }
}
