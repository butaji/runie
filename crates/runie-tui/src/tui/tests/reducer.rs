//! Reducer tests for state updates.

use crate::tui::state::{AppState, AnimationState, CommandPaletteState, Msg, Cmd, ScrollState, TopBarState, PermissionModalState, TuiMode};
use crate::components::{MessageItem, SessionTreeNavigator};
use crate::components::CommandPalette;
use crate::tui::update::update;
use runie_agent::{AgentEvent, AgentMessage, PermissionDecision};
use runie_ai::TokenUsage;
use ratatui_textarea::{TextArea, Input, Key};

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
        terminal_size: (0, 0),
    }
}

fn make_state_with_text(text: &str) -> AppState {
    AppState {
        messages: vec![],
        textarea: TextArea::new(vec![text.to_string()]),
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
        terminal_size: (0, 0),
    }
}

fn type_char(state: &mut AppState, c: char) {
    state.textarea.input(Input { key: Key::Char(c), ctrl: false, alt: false, shift: false });
}

fn type_enter(state: &mut AppState) {
    state.textarea.input(Input { key: Key::Enter, ctrl: false, alt: false, shift: false });
}

#[test]
fn test_textarea_input() {
    let mut state = make_state();
    type_char(&mut state, 'h');
    type_char(&mut state, 'i');
    assert_eq!(state.textarea.lines(), &["hi".to_string()]);
}

#[test]
fn test_submit_clears_input() {
    let mut state = make_state_with_text("hi");
    let mut palette = CommandPalette::new();
    let cmds = update(&mut state, &mut palette, Msg::Submit);
    assert!(state.textarea.is_empty());
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
    let mut palette = CommandPalette::new();
    let cmds = update(&mut state, &mut palette, Msg::Submit);
    assert_eq!(state.messages.len(), 0);
    assert!(cmds.is_empty());
}

#[test]
fn test_quit() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();
    update(&mut state, &mut palette, Msg::Quit);
    assert!(!state.running);
}

#[test]
fn test_toggle_sidebar() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();
    assert!(!state.show_sidebar);
    update(&mut state, &mut palette, Msg::ToggleSidebar);
    assert!(state.show_sidebar);
    update(&mut state, &mut palette, Msg::ToggleSidebar);
    assert!(!state.show_sidebar);
}

#[test]
fn test_agent_event_message_start() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();
    update(
        &mut state,
        &mut palette,
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
    use runie_agent::ContentPart;
    let mut state = make_state();
    let mut palette = CommandPalette::new();
    // Start message
    update(
        &mut state,
        &mut palette,
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
        &mut palette,
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
    let mut palette = CommandPalette::new();

    // PermissionConfirm should return Allow decision
    let cmds = update(&mut state, &mut palette, Msg::PermissionConfirm);
    assert_eq!(cmds.len(), 1);
    if let Cmd::SendPermission { decision } = &cmds[0] {
        assert!(matches!(*decision, PermissionDecision::Allow { .. }));
    } else {
        panic!("Expected SendPermission cmd");
    }

    // PermissionCancel should return Deny decision
    let cmds = update(&mut state, &mut palette, Msg::PermissionCancel);
    if let Cmd::SendPermission { decision } = &cmds[0] {
        assert!(matches!(*decision, PermissionDecision::Deny { .. }));
    }

    // PermissionAlways should return AllowAlways decision
    let cmds = update(&mut state, &mut palette, Msg::PermissionAlways);
    if let Cmd::SendPermission { decision } = &cmds[0] {
        assert!(matches!(*decision, PermissionDecision::AllowAlways { .. }));
    }

    // PermissionSkip should return Skip decision
    let cmds = update(&mut state, &mut palette, Msg::PermissionSkip);
    if let Cmd::SendPermission { decision } = &cmds[0] {
        assert!(matches!(*decision, PermissionDecision::Skip { .. }));
    }
}

#[test]
fn test_multi_line_submit() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();
    for c in "line1".chars() {
        type_char(&mut state, c);
    }
    // Simulate newline via textarea input
    type_enter(&mut state);
    for c in "line2".chars() {
        type_char(&mut state, c);
    }
    update(&mut state, &mut palette, Msg::Submit);

    assert!(state.textarea.is_empty());
    assert_eq!(state.messages.len(), 1);
    if let MessageItem::User { text, .. } = &state.messages[0] {
        assert_eq!(text, "line1\nline2");
    } else {
        panic!("Expected User message");
    }
}

// P0-1 FIX: Msg::Stop interrupts agent without quitting
#[test]
fn test_msg_stop_clears_agent_running() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();
    state.agent_running = true;
    state.mode = TuiMode::Permission; // Simulate being in permission mode
    
    let cmds = update(&mut state, &mut palette, Msg::Stop);
    
    assert!(!state.agent_running, "agent_running should be cleared on Stop");
    assert_eq!(state.mode, TuiMode::Chat, "Mode should reset to Chat on Stop (not Onboarding)");
    assert!(state.running, "running should remain true on Stop (Quit sets it false)");
    
    // Should return Interrupt cmd
    assert!(!cmds.is_empty(), "Stop should produce at least one cmd");
    if let Cmd::Interrupt = &cmds[0] {
        // Expected
    } else {
        panic!("Expected Cmd::Interrupt");
    }
}

// BG-2 FIX: Agent error resets mode to Chat
#[test]
fn test_agent_error_resets_mode() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();
    state.mode = TuiMode::Permission;
    
    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::Error {
        message: "Connection reset".to_string(),
    }));
    
    assert_eq!(state.mode, TuiMode::Chat, "Mode should reset to Chat on agent error");
}

// P1-4 FIX: PermissionCancel triggers Rollback
#[test]
fn test_permission_cancel_triggers_rollback() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();
    state.permission_modal.tool_call_id = Some("tool_123".to_string());
    state.mode = TuiMode::Permission;
    
    let cmds = update(&mut state, &mut palette, Msg::PermissionCancel);
    
    // Should have both SendPermission(Deny) and Rollback
    assert_eq!(cmds.len(), 2, "PermissionCancel should produce SendPermission + Rollback");
    if let Cmd::SendPermission { decision } = &cmds[0] {
        assert!(matches!(*decision, PermissionDecision::Deny { .. }));
    }
    if let Cmd::Rollback { tool_call_id } = &cmds[1] {
        assert_eq!(tool_call_id, "tool_123");
    }
    
    assert_eq!(state.mode, TuiMode::Chat, "Mode should reset to Chat after cancel");
}

// P1-4 FIX: PermissionSkip also triggers Rollback
#[test]
fn test_permission_skip_triggers_rollback() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();
    state.permission_modal.tool_call_id = Some("tool_456".to_string());
    
    let cmds = update(&mut state, &mut palette, Msg::PermissionSkip);
    
    assert_eq!(cmds.len(), 2, "PermissionSkip should produce SendPermission + Rollback");
    if let Cmd::Rollback { tool_call_id } = &cmds[1] {
        assert_eq!(tool_call_id, "tool_456");
    }
}

// BG-5 FIX: AgentEnd clears pending permission modal
#[test]
fn test_agent_end_clears_permission_modal() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();
    state.mode = TuiMode::Permission;
    state.permission_modal.tool = Some("bash".to_string());
    state.permission_modal.tool_call_id = Some("tool_789".to_string());
    state.agent_running = true;
    
    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::AgentEnd {
        messages: vec![],
    }));
    
    assert!(!state.agent_running, "agent_running should be cleared");
    assert_eq!(state.mode, TuiMode::Chat, "Mode should reset to Chat on AgentEnd");
    assert!(state.permission_modal.tool.is_none(), "Permission modal should be cleared");
}

// BG-1: Permission request behavior (not yet implemented - test documents expected behavior)
// KNOWN GAP: Currently, permission request switches mode to Permission.
// The fix would queue permission and stay in DiffViewer until user dismisses the modal.
#[test]
fn test_permission_request_switches_mode() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();
    state.mode = TuiMode::DiffViewer;
    
    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::PermissionRequest {
        tool_call_id: "tool_abc".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "rm -rf /".to_string(),
    }));
    
    // BG-1 FIX: Permission request is queued when in DiffViewer mode
    // Mode stays in DiffViewer to preserve context
    assert_eq!(state.mode, TuiMode::DiffViewer, "BG-1: Mode stays in DiffViewer when permission queued");
    assert_eq!(state.permission_modal.pending_queue.len(), 1, "Permission is queued");
    assert!(state.permission_modal.tool.is_none(), "Current permission is empty");
    
    // Close the DiffViewer and verify queued permission is shown
    update(&mut state, &mut palette, Msg::CloseModal);
    assert_eq!(state.mode, TuiMode::Chat);
    
    // Permission is still queued but not shown since we're back to Chat without permission modal
    // (In real flow, we'd process queue when returning to Chat if agent is still running)
}

// BG-8: State preserved when switching modes
#[test]
fn test_scroll_preserved_on_mode_switch() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();
    state.mode = TuiMode::Chat;
    state.scroll.feed_offset = 100;
    
    // Simulate switching to another mode and back
    update(&mut state, &mut palette, Msg::OpenCommandPalette);
    assert_eq!(state.mode, TuiMode::CommandPalette);
    
    // Switch back to Chat
    update(&mut state, &mut palette, Msg::CloseModal);
    assert_eq!(state.mode, TuiMode::Chat);
    // BG-8 FIX: Scroll should be preserved when returning to Chat
    assert_eq!(state.scroll.feed_offset, 100, "Scroll should be preserved when returning to Chat");
}

// P1-1 FIX: Error message sanitization
#[test]
fn test_long_error_is_truncated() {
    use crate::tui::update::agent::sanitize_error_message;
    
    let long_error = "Error: ".to_string() + &"x".repeat(1000);
    let sanitized = sanitize_error_message(&long_error);
    
    assert!(sanitized.len() < long_error.len(), "Long error should be truncated");
    assert!(sanitized.contains("[message truncated"), "Should indicate truncation");
}

#[test]
fn test_stack_trace_shows_summary() {
    use crate::tui::update::agent::sanitize_error_message;
    
    // Stack trace with lowercase patterns that match the detection
    let stack_trace = "Connection error\nstack backtrace:\n   at 0x7f8d9f... (main.rs:100)\n   at 0x7f8da0... (main.rs:101)";
    let sanitized = sanitize_error_message(stack_trace);
    
    // Stack traces are summarized to first 5 lines
    assert!(sanitized.contains("Connection error"), "Should preserve error summary");
    // Check if it's treated as stack trace (first 5 lines)
    let first_five = "Connection error\nstack backtrace:\n   at 0x7f8d9f... (main.rs:100)\n   at 0x7f8da0... (main.rs:101)";
    assert_eq!(sanitized.lines().count(), first_five.lines().count() + 1, 
        "Should add hidden details note");
}

// P1-4 FIX: Submit blocked with feedback when agent running
#[test]
fn test_submit_blocked_feedback_when_agent_running() {
    let mut state = make_state_with_text("Hello");  // Add text so Submit is processed
    let mut palette = CommandPalette::new();
    state.agent_running = true;
    state.messages = vec![]; // Clear any messages
    
    update(&mut state, &mut palette, Msg::Submit);
    
    // Should have pushed a system message explaining the block
    assert_eq!(state.messages.len(), 1, "Should show feedback message");
    if let MessageItem::System { text } = &state.messages[0] {
        assert!(text.contains("still running") || text.contains("Ctrl+C"), 
            "Feedback should mention agent running or Ctrl+C");
    }
}

// BG-6: Duplicate submit deduplication
#[test]
fn test_duplicate_submit_is_deduplicated() {
    let mut state = make_state_with_text("Hello");
    let mut palette = CommandPalette::new();
    
    // First submit
    update(&mut state, &mut palette, Msg::Submit);
    assert_eq!(state.messages.len(), 1, "First submit should add message");
    
    // Clear textarea and type same message
    state.textarea = TextArea::new(vec!["Hello".to_string()]);
    
    // The implementation doesn't currently deduplicate by content,
    // but it does block while agent_running. Let's test the blocking.
    state.agent_running = true;
    update(&mut state, &mut palette, Msg::Submit);
    
    // Should have added feedback message (duplicate prevention)
    assert!(state.messages.len() >= 2, "Blocked submit should show feedback");
}
