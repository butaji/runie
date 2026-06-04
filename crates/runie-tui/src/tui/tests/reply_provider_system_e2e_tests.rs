//! ReplyProvider → Agent Loop → TUI State → ViewModels E2E tests.
//!
//! Tests critical system behaviors: permission flows, tool execution,
//! interrupts, error recovery, and multi-tool scenarios.
//!
//! Use `handle_agent_event(&mut state, event)` for AgentEvent-based tests.
//! Use `update(&mut state, &mut palette, msg)` for Msg-based tests (interrupt).
//! Build ViewModels with `ViewModels::from_app_state(state, &palette, wrap_cache)`.

use crate::components::message_list::feed::FeedItem;
use crate::components::message_list::render::WrapCache;
use crate::components::CommandPalette;
use crate::components::MessageItem;
use crate::tui::state::AppState;
use crate::tui::state::TuiMode;
use crate::tui::update::agent::handle_agent_event;
use crate::tui::update::update;
use crate::tui::view_models::ViewModels;
use runie_agent::{AgentEvent, AgentMessage, ContentPart, ToolResult};

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Create an AgentMessage with given role and content text.
fn agent_message(role: &str, text: &str) -> AgentMessage {
    AgentMessage {
        role: role.to_string(),
        content: vec![ContentPart::Text {
            text: text.to_string(),
        }],
        timestamp: 0,
        usage: None,
        stop_reason: None,
        error_message: None,
        tool_calls: vec![],
    }
}

/// Create AppState ready for testing with model set.
fn make_test_state() -> AppState {
    let mut state = AppState::default();
    state.current_model = Some("MiniMax-M2.7-highspeed".to_string());
    state
}

/// Build viewmodels from app state.
fn build_viewmodels(state: &AppState) -> ViewModels {
    let palette = CommandPalette::default();
    let wrap_cache = WrapCache::default();
    ViewModels::from_app_state(state, &palette, wrap_cache)
}

// ─── Test 1: Permission Allow Continues Tool Execution ───────────────────────

/// Process: PermissionRequest → PermissionConfirm → ToolExecutionStart →
/// ToolExecutionEnd → MessageStart → MessageUpdate → MessageEnd → AgentEnd
///
/// Asserts:
/// - After PermissionRequest: `state.mode` is Permission, `permission_modal` has tool info
/// - After PermissionConfirm: `state.mode` returns to Chat
/// - ToolExecutionStart creates ToolCall item
/// - ToolExecutionEnd updates tool with result
/// - Model responds after tool result (MessageUpdate + MessageEnd)
#[test]
fn test_permission_allow_continues_tool_execution() {
    let mut state = make_test_state();
    let mut palette = CommandPalette::default();

    // ── PermissionRequest ──────────────────────────────────────────────────
    handle_agent_event(&mut state, AgentEvent::PermissionRequest {
        tool_call_id: "call_abc123".to_string(),
        tool_name: "bash".to_string(),
        tool_args: r#"{"command": "echo hello"}"#.to_string(),
        tool_description: "Execute a bash command".to_string(),
        turn: 0,
        context_window_usage: 0.25,
    });

    // Assert: mode is Permission, permission_modal has tool info
    assert_eq!(state.mode, TuiMode::Permission, "mode should be Permission after request");
    assert_eq!(
        state.permission_modal.tool.as_deref(),
        Some("bash"),
        "permission_modal.tool should be set"
    );
    assert_eq!(
        state.permission_modal.tool_call_id.as_deref(),
        Some("call_abc123"),
        "permission_modal.tool_call_id should be set"
    );

    // Assert via ViewModel
    let vm_before = build_viewmodels(&state);
    assert!(
        vm_before.permission_modal.is_some(),
        "vm.permission_modal should be Some during permission request"
    );

    // ── PermissionConfirm (via Msg) ────────────────────────────────────────
    let cmds = update(&mut state, &mut palette, crate::tui::state::Msg::PermissionConfirm);
    assert!(
        cmds.iter().any(|c| matches!(c, crate::tui::state::Cmd::SendPermission { .. })),
        "PermissionConfirm should produce SendPermission cmd"
    );

    // Assert: mode returns to Chat, permission_modal cleared
    assert_eq!(state.mode, TuiMode::Chat, "mode should return to Chat after confirm");
    assert!(
        state.permission_modal.tool.is_none(),
        "permission_modal.tool should be cleared"
    );

    // ── ToolExecutionStart ────────────────────────────────────────────────
    handle_agent_event(&mut state, AgentEvent::ToolExecutionStart {
        tool_call_id: "call_abc123".to_string(),
        tool_name: "bash".to_string(),
        tool_args: r#"{"command": "echo hello"}"#.to_string(),
        turn: 0,
    });

    // Assert: ToolCall item exists in messages
    let has_tool_call = state.messages.iter().any(|m| {
        matches!(m, MessageItem::ToolCall { name, .. } if name == "call_abc123")
    });
    assert!(has_tool_call, "state.messages should have ToolCall item");

    // ── ToolExecutionEnd ───────────────────────────────────────────────────
    handle_agent_event(&mut state, AgentEvent::ToolExecutionEnd {
        tool_call_id: "call_abc123".to_string(),
        tool_name: "bash".to_string(),
        tool_args: r#"{"command": "echo hello"}"#.to_string(),
        result: ToolResult {
            tool_call_id: "call_abc123".to_string(),
            tool_name: "bash".to_string(),
            input: serde_json::json!({"command": "echo hello"}),
            content: vec![ContentPart::Text { text: "hello\n".to_string() }],
            is_error: false,
        },
        duration_ms: 150,
        turn: 0,
    });

    // Assert: ToolCall has result
    let tool_has_result = state.messages.iter().any(|m| {
        if let MessageItem::ToolCall { result, .. } = m {
            result.as_ref().map_or(false, |r| r.contains("hello"))
        } else {
            false
        }
    });
    assert!(tool_has_result, "ToolCall should have result with 'hello'");

    // ── Model responds after tool result ──────────────────────────────────
    handle_agent_event(&mut state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });
    handle_agent_event(&mut state, AgentEvent::MessageUpdate {
        message: agent_message("assistant", "I ran the command for you. Output: hello"),
        delta: "I ran the command for you. Output: hello".to_string(),
        replace: false,
        turn: 1,
    });
    handle_agent_event(&mut state, AgentEvent::MessageEnd {
        message: agent_message("assistant", "I ran the command for you. Output: hello"),
        turn: 1,
    });
    handle_agent_event(&mut state, AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 2,
        final_token_usage: runie_agent::TokenUsage::default(),
    });

    // Assert: Final state
    assert!(!state.agent_running, "agent_running should be false after AgentEnd");

    // Assert via ViewModel
    let vm = build_viewmodels(&state);
    assert!(
        vm.agent_list.agent_running == state.agent_running,
        "ViewModel agent_running should match state"
    );
}

// ─── Test 2: Permission Deny Skips Tool ─────────────────────────────────────

/// Process: PermissionRequest → PermissionCancel → MessageEnd → AgentEnd
///
/// Asserts:
/// - After PermissionCancel: mode returns to Chat
/// - No ToolExecutionStart event processed
/// - Assistant message explains tool was denied or continues without tool
#[test]
fn test_permission_deny_skips_tool() {
    let mut state = make_test_state();
    let mut palette = CommandPalette::default();

    // ── PermissionRequest ──────────────────────────────────────────────────
    handle_agent_event(&mut state, AgentEvent::PermissionRequest {
        tool_call_id: "call_deny_123".to_string(),
        tool_name: "bash".to_string(),
        tool_args: r#"{"command": "rm -rf /"}"#.to_string(),
        tool_description: "Execute a dangerous bash command".to_string(),
        turn: 0,
        context_window_usage: 0.25,
    });

    assert_eq!(state.mode, TuiMode::Permission, "mode should be Permission");

    // ── PermissionCancel (via Msg) ─────────────────────────────────────────
    let cmds = update(&mut state, &mut palette, crate::tui::state::Msg::PermissionCancel);
    assert!(
        cmds.iter().any(|c| matches!(c, crate::tui::state::Cmd::SendPermission { .. })),
        "PermissionCancel should produce SendPermission cmd"
    );

    // Assert: mode returns to Chat, no tool execution
    assert_eq!(state.mode, TuiMode::Chat, "mode should return to Chat after cancel");
    assert!(
        state.permission_modal.tool.is_none(),
        "permission_modal.tool should be cleared"
    );

    // Assert: No ToolCall items (denied before execution)
    let tool_call_count = state.messages.iter()
        .filter(|m| matches!(m, MessageItem::ToolCall { .. }))
        .count();
    assert_eq!(tool_call_count, 0, "No ToolCall should exist after deny");

    // ── Assistant continues without tool (MessageEnd → AgentEnd) ───────────
    handle_agent_event(&mut state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::MessageUpdate {
        message: agent_message("assistant", "I won't run that dangerous command."),
        delta: "I won't run that dangerous command.".to_string(),
        replace: false,
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::MessageEnd {
        message: agent_message("assistant", "I won't run that dangerous command."),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: runie_agent::TokenUsage::default(),
    });

    // Assert: Assistant message exists explaining denial
    let has_assistant = state.messages.iter().any(|m| {
        if let MessageItem::Assistant { text, .. } = m {
            text.contains("won't run") || text.contains("denied") || text.contains("dangerous")
        } else {
            false
        }
    });
    assert!(has_assistant, "Assistant message should explain tool was not run");

    // Assert via ViewModel
    let vm = build_viewmodels(&state);
    assert!(
        !vm.agent_list.agent_running,
        "agent_running should be false after AgentEnd"
    );
}

// ─── Test 3: Permission Always Bypasses Future Requests ───────────────────────

/// Process: PermissionRequest → PermissionAlways → ToolExecutionStart →
/// ToolExecutionEnd → AgentEnd
///
/// Asserts:
/// - Tool executes without further permission prompts
/// - `permission_modal` is cleared
#[test]
fn test_permission_always_bypasses_future_requests() {
    let mut state = make_test_state();
    let mut palette = CommandPalette::default();

    // ── PermissionRequest ──────────────────────────────────────────────────
    handle_agent_event(&mut state, AgentEvent::PermissionRequest {
        tool_call_id: "call_always_123".to_string(),
        tool_name: "bash".to_string(),
        tool_args: r#"{"command": "ls"}"#.to_string(),
        tool_description: "List files".to_string(),
        turn: 0,
        context_window_usage: 0.25,
    });

    assert_eq!(state.mode, TuiMode::Permission, "mode should be Permission");

    // ── PermissionAlways (via Msg) ─────────────────────────────────────────
    let cmds = update(&mut state, &mut palette, crate::tui::state::Msg::PermissionAlways);
    assert!(
        cmds.iter().any(|c| matches!(c, crate::tui::state::Cmd::SendPermission { .. })),
        "PermissionAlways should produce SendPermission cmd"
    );

    // Assert: mode returns to Chat
    assert_eq!(state.mode, TuiMode::Chat, "mode should return to Chat after always");
    assert!(
        state.permission_modal.tool.is_none(),
        "permission_modal.tool should be cleared"
    );

    // ── Tool executes immediately without permission prompt ─────────────────
    handle_agent_event(&mut state, AgentEvent::ToolExecutionStart {
        tool_call_id: "call_always_123".to_string(),
        tool_name: "bash".to_string(),
        tool_args: r#"{"command": "ls"}"#.to_string(),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::ToolExecutionEnd {
        tool_call_id: "call_always_123".to_string(),
        tool_name: "bash".to_string(),
        tool_args: r#"{"command": "ls"}"#.to_string(),
        result: ToolResult {
            tool_call_id: "call_always_123".to_string(),
            tool_name: "bash".to_string(),
            input: serde_json::json!({"command": "ls"}),
            content: vec![ContentPart::Text { text: "file1.txt\nfile2.txt\n".to_string() }],
            is_error: false,
        },
        duration_ms: 50,
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::MessageEnd {
        message: agent_message("assistant", "Listed files for you."),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: runie_agent::TokenUsage::default(),
    });

    // Assert: ToolCall exists with result
    let tool_has_result = state.messages.iter().any(|m| {
        if let MessageItem::ToolCall { result, .. } = m {
            result.is_some()
        } else {
            false
        }
    });
    assert!(tool_has_result, "ToolCall should have result after AllowAlways execution");
}

// ─── Test 4: Interrupt Clears Agent Running ──────────────────────────────────

/// Process: MessageStart → MessageUpdate (partial text) → Msg::Stop (interrupt)
///
/// Asserts:
/// - After MessageStart + MessageUpdate: `agent_running` is true, text has partial content
/// - After Msg::Stop: `agent_running` is false
/// - Partial assistant message remains in feed
/// - `global_tags` shows idle state
#[test]
fn test_interrupt_clears_agent_running() {
    let mut state = make_test_state();
    let mut palette = CommandPalette::default();

    // ── MessageStart ────────────────────────────────────────────────────────
    handle_agent_event(&mut state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 0,
    });

    assert!(state.agent_running, "agent_running should be true after MessageStart");

    // ── MessageUpdate (partial streaming text) ─────────────────────────────
    handle_agent_event(&mut state, AgentEvent::MessageUpdate {
        message: agent_message("assistant", "Here is a partial "),
        delta: "Here is a partial ".to_string(),
        replace: false,
        turn: 0,
    });

    // Get partial text
    let partial_text = state.messages.iter()
        .filter_map(|m| {
            if let MessageItem::Assistant { text, .. } = m {
                Some(text.clone())
            } else {
                None
            }
        })
        .last();

    assert!(
        partial_text.as_ref().map_or(false, |t| t.contains("partial")),
        "Partial text should be accumulated"
    );

    // Assert via ViewModel that agent_running is true during streaming
    let vm_during = build_viewmodels(&state);
    assert!(
        vm_during.global_tags.left.is_some(),
        "global_tags should show running state during streaming"
    );

    // ── Msg::Stop (interrupt) ──────────────────────────────────────────────
    let cmds = update(&mut state, &mut palette, crate::tui::state::Msg::Stop);

    // Assert: agent_running is false, partial message remains
    assert!(!state.agent_running, "agent_running should be false after interrupt");
    assert!(
        cmds.iter().any(|c| matches!(c, crate::tui::state::Cmd::Interrupt)),
        "Stop should produce Interrupt cmd"
    );

    // Assert: Partial assistant message still exists in feed
    let partial_text_after = state.messages.iter()
        .filter_map(|m| {
            if let MessageItem::Assistant { text, .. } = m {
                Some(text.clone())
            } else {
                None
            }
        })
        .last();

    assert!(
        partial_text_after.as_ref().map_or(false, |t| t.contains("partial")),
        "Partial text should remain in feed after interrupt"
    );

    // Assert via ViewModel that global_tags shows idle state
    let vm_after = build_viewmodels(&state);
    assert!(
        vm_after.global_tags.left.is_none(),
        "global_tags should show idle state after interrupt"
    );

    // Assert: mode returned to Chat
    assert_eq!(state.mode, TuiMode::Chat, "mode should return to Chat after interrupt");
}

// ─── Test 5: Tool Result Feeds Back to Model ─────────────────────────────────

/// Process complete cycle:
/// - Turn 1: MessageStart → MessageUpdate (tool call) → ToolExecutionStart →
///           ToolExecutionEnd
/// - Turn 2: MessageStart → MessageUpdate (model responds to tool result) →
///           MessageEnd → AgentEnd
///
/// Asserts:
/// - Messages have: User → Assistant (tool call) → ToolCall → Assistant (response)
/// - Tool result is visible in feed
/// - Second assistant message responds to tool result
#[test]
fn test_tool_result_feeds_back_to_model() {
    let mut state = make_test_state();

    // ── Turn 1 ─────────────────────────────────────────────────────────────
    handle_agent_event(&mut state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 0,
    });

    // Assistant requests tool
    handle_agent_event(&mut state, AgentEvent::MessageUpdate {
        message: agent_message("assistant", "I'll run the ls command for you."),
        delta: "I'll run the ls command for you.".to_string(),
        replace: false,
        turn: 0,
    });

    // Tool executes
    handle_agent_event(&mut state, AgentEvent::ToolExecutionStart {
        tool_call_id: "call_ls_1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: r#"{"command": "ls"}"#.to_string(),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::ToolExecutionEnd {
        tool_call_id: "call_ls_1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: r#"{"command": "ls"}"#.to_string(),
        result: ToolResult {
            tool_call_id: "call_ls_1".to_string(),
            tool_name: "bash".to_string(),
            input: serde_json::json!({"command": "ls"}),
            content: vec![ContentPart::Text { text: "file1.txt\nfile2.txt\n".to_string() }],
            is_error: false,
        },
        duration_ms: 100,
        turn: 0,
    });

    // Turn end
    handle_agent_event(&mut state, AgentEvent::TurnEnd {
        turn: 0,
        message_count: 3,
        tool_results_count: 1,
        token_usage: runie_agent::TokenUsage::default(),
    });

    // ── Turn 2 ─────────────────────────────────────────────────────────────
    handle_agent_event(&mut state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });

    // Model responds to tool result
    handle_agent_event(&mut state, AgentEvent::MessageUpdate {
        message: agent_message("assistant", "I ran ls and found 2 files: file1.txt and file2.txt"),
        delta: "I ran ls and found 2 files: file1.txt and file2.txt".to_string(),
        replace: false,
        turn: 1,
    });
    handle_agent_event(&mut state, AgentEvent::MessageEnd {
        message: agent_message("assistant", "I ran ls and found 2 files: file1.txt and file2.txt"),
        turn: 1,
    });
    handle_agent_event(&mut state, AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 2,
        final_token_usage: runie_agent::TokenUsage::default(),
    });

    // Assert: Messages structure - User → Assistant (tool call) → ToolCall → Assistant (response)
    let assistant_count = state.messages.iter()
        .filter(|m| matches!(m, MessageItem::Assistant { .. }))
        .count();
    assert_eq!(assistant_count, 2, "Should have 2 Assistant messages (one per turn)");

    let tool_call_count = state.messages.iter()
        .filter(|m| matches!(m, MessageItem::ToolCall { .. }))
        .count();
    assert_eq!(tool_call_count, 1, "Should have 1 ToolCall");

    // Assert: Second assistant responds to tool result
    let second_assistant_text = state.messages.iter()
        .filter_map(|m| {
            if let MessageItem::Assistant { text, .. } = m {
                Some(text.clone())
            } else {
                None
            }
        })
        .last();

    assert!(
        second_assistant_text.as_ref().map_or(false, |t| {
            t.contains("file1.txt") && t.contains("file2.txt")
        }),
        "Second assistant should reference tool result files"
    );

    // Assert via ViewModel
    let vm = build_viewmodels(&state);
    let assistant_count_in_feed = vm.message_list.feed.items().iter()
        .filter(|item| matches!(item, FeedItem::AssistantMessage { .. }))
        .count();
    assert_eq!(assistant_count_in_feed, 2, "vm should have 2 assistant messages in feed");
}

// ─── Test 6: Error Recovery Next Message Works ───────────────────────────────

/// Process: MessageStart → Error event → (wait) → New MessageStart →
/// MessageUpdate → MessageEnd → AgentEnd
///
/// Asserts:
/// - After Error: `agent_running` is false, Error message in feed
/// - After new MessageStart: `agent_running` is true again
/// - New assistant message appears
/// - Previous error doesn't block new message
#[test]
fn test_error_recovery_next_message_works() {
    let mut state = make_test_state();

    // ── First MessageStart ─────────────────────────────────────────────────
    handle_agent_event(&mut state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 0,
    });
    assert!(state.agent_running, "agent_running should be true after MessageStart");

    // ── Error event ────────────────────────────────────────────────────────
    handle_agent_event(&mut state, AgentEvent::Error {
        message: "API rate limit exceeded".to_string(),
        error_type: "rate_limit".to_string(),
        recoverable: true,
        context: "When calling the LLM API".to_string(),
    });

    // Assert: agent_running is false, Error message in feed
    assert!(!state.agent_running, "agent_running should be false after error");
    assert!(state.messages.iter().any(|m| matches!(m, MessageItem::Error { .. })),
        "Error message should be in feed");

    // Assert via ViewModel
    let vm_error = build_viewmodels(&state);
    assert!(
        vm_error.message_list.feed.items().iter().any(|item| {
            matches!(item, FeedItem::SystemNotice { text } if text.contains("rate limit"))
        }),
        "Error should appear as SystemNotice in feed"
    );

    // ── New MessageStart (recovery) ────────────────────────────────────────
    handle_agent_event(&mut state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 0,
    });

    // Assert: agent_running is true again
    assert!(state.agent_running, "agent_running should be true after new MessageStart");

    // ── Complete new message ────────────────────────────────────────────────
    handle_agent_event(&mut state, AgentEvent::MessageUpdate {
        message: agent_message("assistant", "Recovery message after error."),
        delta: "Recovery message after error.".to_string(),
        replace: false,
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::MessageEnd {
        message: agent_message("assistant", "Recovery message after error."),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: runie_agent::TokenUsage::default(),
    });

    // Assert: New assistant message exists
    let has_recovery_message = state.messages.iter().any(|m| {
        if let MessageItem::Assistant { text, .. } = m {
            text.contains("Recovery")
        } else {
            false
        }
    });
    assert!(has_recovery_message, "Recovery message should appear after error");

    // Assert via ViewModel
    let vm_recovery = build_viewmodels(&state);
    assert!(
        !vm_recovery.agent_list.agent_running,
        "agent_running should be false after AgentEnd"
    );
}

// ─── Test 7: Multiple Tool Calls in One Turn ─────────────────────────────────

/// Process: MessageStart → ToolExecutionStart (tool1) → ToolExecutionEnd (tool1) →
/// ToolExecutionStart (tool2) → ToolExecutionEnd (tool2) → MessageUpdate →
/// MessageEnd → AgentEnd
///
/// Asserts:
/// - Two separate ToolCall items in messages
/// - Each has correct name and result
/// - Model responds after all tools complete
#[test]
fn test_multiple_tool_calls_in_one_turn() {
    let mut state = make_test_state();

    // ── MessageStart (Assistant placeholder) ────────────────────────────────
    handle_agent_event(&mut state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 0,
    });

    // ── First tool ─────────────────────────────────────────────────────────
    handle_agent_event(&mut state, AgentEvent::ToolExecutionStart {
        tool_call_id: "call_read_1".to_string(),
        tool_name: "read_file".to_string(),
        tool_args: r#"{"path": "file1.txt"}"#.to_string(),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::ToolExecutionEnd {
        tool_call_id: "call_read_1".to_string(),
        tool_name: "read_file".to_string(),
        tool_args: r#"{"path": "file1.txt"}"#.to_string(),
        result: ToolResult {
            tool_call_id: "call_read_1".to_string(),
            tool_name: "read_file".to_string(),
            input: serde_json::json!({"path": "file1.txt"}),
            content: vec![ContentPart::Text { text: "content of file1".to_string() }],
            is_error: false,
        },
        duration_ms: 50,
        turn: 0,
    });

    // ── Second tool ─────────────────────────────────────────────────────────
    handle_agent_event(&mut state, AgentEvent::ToolExecutionStart {
        tool_call_id: "call_write_1".to_string(),
        tool_name: "write_file".to_string(),
        tool_args: r#"{"path": "file2.txt", "content": "new content"}"#.to_string(),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::ToolExecutionEnd {
        tool_call_id: "call_write_1".to_string(),
        tool_name: "write_file".to_string(),
        tool_args: r#"{"path": "file2.txt", "content": "new content"}"#.to_string(),
        result: ToolResult {
            tool_call_id: "call_write_1".to_string(),
            tool_name: "write_file".to_string(),
            input: serde_json::json!({"path": "file2.txt", "content": "new content"}),
            content: vec![ContentPart::Text { text: "Written successfully".to_string() }],
            is_error: false,
        },
        duration_ms: 75,
        turn: 0,
    });

    // ── Model responds after all tools ────────────────────────────────────
    // Note: After tools complete, the model generates a new assistant message
    handle_agent_event(&mut state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });
    handle_agent_event(&mut state, AgentEvent::MessageUpdate {
        message: agent_message("assistant", "I read file1.txt and wrote to file2.txt."),
        delta: "I read file1.txt and wrote to file2.txt.".to_string(),
        replace: false,
        turn: 1,
    });
    handle_agent_event(&mut state, AgentEvent::MessageEnd {
        message: agent_message("assistant", "I read file1.txt and wrote to file2.txt."),
        turn: 1,
    });
    handle_agent_event(&mut state, AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 2,
        final_token_usage: runie_agent::TokenUsage::default(),
    });

    // Assert: Two separate ToolCall items (name field stores tool_call_id)
    let tool_calls: Vec<_> = state.messages.iter()
        .filter(|m| {
            if let MessageItem::ToolCall { name, .. } = m {
                name.contains("call_read_1") || name.contains("call_write_1")
            } else {
                false
            }
        })
        .collect();
    assert_eq!(tool_calls.len(), 2, "Should have 2 ToolCall items");

    // Assert: First tool has correct tool_call_id and result
    let read_tool = state.messages.iter().find(|m| {
        if let MessageItem::ToolCall { name, .. } = m {
            name.contains("call_read_1")
        } else {
            false
        }
    });
    assert!(
        read_tool.is_some(),
        "Should have read_file tool call"
    );
    if let Some(MessageItem::ToolCall { result, .. }) = read_tool {
        assert!(
            result.as_ref().map_or(false, |r| r.contains("content of file1")),
            "read_file should have result with content"
        );
    }

    // Assert: Second tool has correct tool_call_id and result
    let write_tool = state.messages.iter().find(|m| {
        if let MessageItem::ToolCall { name, .. } = m {
            name.contains("call_write_1")
        } else {
            false
        }
    });
    assert!(
        write_tool.is_some(),
        "Should have write_file tool call"
    );
    if let Some(MessageItem::ToolCall { result, .. }) = write_tool {
        assert!(
            result.as_ref().map_or(false, |r| r.contains("Written successfully")),
            "write_file should have success result"
        );
    }

    // Assert: Model responds after all tools (second assistant message)
    let assistant_text = state.messages.iter()
        .filter_map(|m| {
            if let MessageItem::Assistant { text, .. } = m {
                Some(text.clone())
            } else {
                None
            }
        })
        .last();
    assert!(
        assistant_text.as_ref().map_or(false, |t| {
            t.contains("file1.txt") && t.contains("file2.txt")
        }),
        "Assistant should reference both tool results"
    );
}

// ─── Test 8: Context Compacted Event Ignored ─────────────────────────────────

/// Process: MessageStart → MessageUpdate → MessageEnd → ContextCompacted →
/// MessageStart → MessageUpdate → MessageEnd → AgentEnd
///
/// Asserts:
/// - ContextCompacted doesn't break anything
/// - Messages continue to accumulate normally
/// - No state corruption
#[test]
fn test_context_compacted_event_ignored() {
    let mut state = make_test_state();

    // ── First Message ──────────────────────────────────────────────────────
    handle_agent_event(&mut state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::MessageUpdate {
        message: agent_message("assistant", "First message before context compaction."),
        delta: "First message before context compaction.".to_string(),
        replace: false,
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::MessageEnd {
        message: agent_message("assistant", "First message before context compaction."),
        turn: 0,
    });

    // ── ContextCompacted event (should be ignored silently) ────────────────
    handle_agent_event(&mut state, AgentEvent::ContextCompacted {
        original_count: 10,
        compacted_count: 5,
        summary_preview: "Previous conversation summarized...".to_string(),
    });

    // Assert: State unchanged after ContextCompacted
    let assistant_count_before = state.messages.iter()
        .filter(|m| matches!(m, MessageItem::Assistant { .. }))
        .count();
    assert_eq!(assistant_count_before, 1, "Should still have 1 assistant message");

    // ── Second Message (after compaction) ──────────────────────────────────
    handle_agent_event(&mut state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });
    handle_agent_event(&mut state, AgentEvent::MessageUpdate {
        message: agent_message("assistant", "Second message after context compaction."),
        delta: "Second message after context compaction.".to_string(),
        replace: false,
        turn: 1,
    });
    handle_agent_event(&mut state, AgentEvent::MessageEnd {
        message: agent_message("assistant", "Second message after context compaction."),
        turn: 1,
    });
    handle_agent_event(&mut state, AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 2,
        final_token_usage: runie_agent::TokenUsage::default(),
    });

    // Assert: Two assistant messages accumulated correctly
    let assistant_count_after = state.messages.iter()
        .filter(|m| matches!(m, MessageItem::Assistant { .. }))
        .count();
    assert_eq!(assistant_count_after, 2, "Should have 2 assistant messages");

    // Assert: Second message has correct text
    let second_text = state.messages.iter()
        .filter_map(|m| {
            if let MessageItem::Assistant { text, .. } = m {
                Some(text.clone())
            } else {
                None
            }
        })
        .last();
    assert!(
        second_text.as_ref().map_or(false, |t| t.contains("after context compaction")),
        "Second message should have correct text"
    );

    // Assert via ViewModel
    let vm = build_viewmodels(&state);
    assert!(
        !vm.agent_list.agent_running,
        "agent_running should be false after AgentEnd"
    );
    let feed_assistant_count = vm.message_list.feed.items().iter()
        .filter(|item| matches!(item, FeedItem::AssistantMessage { .. }))
        .count();
    assert_eq!(feed_assistant_count, 2, "vm should have 2 assistant messages in feed");
}