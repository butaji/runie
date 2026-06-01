//! ReplyProvider → Agent Loop → TUI State → ViewModels E2E tests.
//!
//! These tests verify the complete flow from AgentEvent emission through
//! AppState mutation to final ViewModel construction.

use crate::components::message_list::feed::FeedItem;
use crate::components::MessageItem;
use crate::tui::state::AppState;
use crate::tui::update::agent::handle_agent_event;
use crate::tui::view_models::ViewModels;
use crate::components::message_list::render::WrapCache;
use crate::components::CommandPalette;
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

// ─── Test 1: Simple Message E2E ───────────────────────────────────────────────

/// Process: MessageStart → MessageUpdate (with text) → MessageEnd → AgentEnd
#[test]
fn test_simple_message_e2e() {
    let mut state = make_test_state();

    // Process full event sequence
    handle_agent_event(&mut state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::MessageUpdate {
        message: agent_message("assistant", "Hello! How can I help you today?"),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::MessageEnd {
        message: agent_message("assistant", "Hello! How can I help you today?"),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: runie_agent::TokenUsage::default(),
    });

    // Assert state
    assert!(!state.agent_running, "agent_running should be false after AgentEnd");
    assert!(state.messages.iter().any(|m| matches!(m, MessageItem::Assistant { .. })),
        "state.messages should have Assistant message");

    // Build viewmodels and assert
    let vm = build_viewmodels(&state);

    assert!(vm.message_list.feed.items().iter().any(|item| {
        matches!(item, FeedItem::AssistantMessage { text, .. } if text.contains("Hello"))
    }), "vm.message_list should show assistant message with Hello");

    assert_eq!(
        vm.status_bar.current_model.as_deref(),
        Some("MiniMax-M2.7-highspeed"),
        "vm.status_bar should show correct model"
    );
}

// ─── Test 2: Tool Call E2E ────────────────────────────────────────────────────

/// Process: MessageStart → ToolExecutionStart → ToolExecutionEnd → MessageEnd → AgentEnd
#[test]
fn test_tool_call_e2e() {
    let mut state = make_test_state();

    handle_agent_event(&mut state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::ToolExecutionStart {
        tool_call_id: "call_abc123".to_string(),
        tool_name: "bash".to_string(),
        tool_args: r#"{"command": "echo hello"}"#.to_string(),
        turn: 0,
    });
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
    handle_agent_event(&mut state, AgentEvent::MessageUpdate {
        message: agent_message("assistant", "I ran the command for you."),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::MessageEnd {
        message: agent_message("assistant", "I ran the command for you."),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: runie_agent::TokenUsage::default(),
    });

    // Assert state has ToolCall - name stores tool_call_id, result stores output
    let has_tool_call = state.messages.iter().any(|m| {
        if let MessageItem::ToolCall { name, result, .. } = m {
            // name is the tool_call_id
            name.contains("call_abc123") && result.as_ref().map_or(false, |r| r.contains("hello"))
        } else {
            false
        }
    });
    assert!(has_tool_call, "state.messages should have ToolCall with correct name and args");

    // Build viewmodels and assert
    let vm = build_viewmodels(&state);

    // ToolCalls are attached inline to AssistantMessage in the feed
    assert!(vm.message_list.feed.items().iter().any(|item| {
        matches!(item, FeedItem::AssistantMessage { .. })
    }), "vm.message_list should show assistant message");
}

// ─── Test 3: Streaming Accumulation E2E ──────────────────────────────────────

/// Process: Multiple MessageUpdate events (chunk1, chunk2, chunk3) →
/// MessageEnd → AgentEnd
#[test]
fn test_streaming_accumulation_e2e() {
    let mut state = make_test_state();

    handle_agent_event(&mut state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 0,
    });
    // Simulate streaming: multiple chunks arrive
    handle_agent_event(&mut state, AgentEvent::MessageUpdate {
        message: agent_message("assistant", "First chunk: "),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::MessageUpdate {
        message: agent_message("assistant", "Second chunk: "),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::MessageUpdate {
        message: agent_message("assistant", "Third chunk complete!"),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::MessageEnd {
        message: agent_message("assistant", "First chunk: Second chunk: Third chunk complete!"),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: runie_agent::TokenUsage::default(),
    });

    // Find the assistant message
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
        assistant_text.as_ref().map_or(false, |t| t.contains("First chunk") && t.contains("Second chunk") && t.contains("Third chunk")),
        "Assistant text should be concatenation of all chunks"
    );

    // Build viewmodels and assert
    let vm = build_viewmodels(&state);

    let has_complete_message = vm.message_list.feed.items().iter().any(|item| {
        if let FeedItem::AssistantMessage { text, .. } = item {
            text.contains("First chunk") && text.contains("Second chunk") && text.contains("Third chunk")
        } else {
            false
        }
    });
    assert!(has_complete_message, "vm.message_list should show complete accumulated message");
}

// ─── Test 4: Thinking Separation E2E ─────────────────────────────────────────

/// Process: MessageStart → ThinkingUpdate ("thinking...") → MessageUpdate ("response")
/// → MessageEnd → AgentEnd
#[test]
fn test_thinking_separation_e2e() {
    let mut state = make_test_state();

    handle_agent_event(&mut state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::ThinkingUpdate {
        text: "Let me think about this...".to_string(),
        turn: 0,
    });
    // Need ThinkingEnd to push the Thought - with duration > 0.5s to pass the check
    std::thread::sleep(std::time::Duration::from_millis(600));
    handle_agent_event(&mut state, AgentEvent::ThinkingEnd {
        duration_ms: 600,
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::MessageUpdate {
        message: agent_message("assistant", "Here's my response!"),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::MessageEnd {
        message: agent_message("assistant", "Here's my response!"),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: runie_agent::TokenUsage::default(),
    });

    // Assert state has separate Thought and Assistant items
    let has_thought = state.messages.iter().any(|m| matches!(m, MessageItem::Thought { .. }));
    let has_assistant = state.messages.iter().any(|m| matches!(m, MessageItem::Assistant { .. }));

    assert!(has_thought, "state.messages should have separate Thought item");
    assert!(has_assistant, "state.messages should have Assistant item");

    // Verify thinking text is NOT in assistant text
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
        assistant_text.as_ref().map_or(false, |t| !t.contains("Let me think")),
        "Thinking text should NOT be in assistant text"
    );

    // Build viewmodels and assert
    let vm = build_viewmodels(&state);

    // Thought is filtered out in Feed (now inline), but assistant should exist
    assert!(vm.message_list.feed.items().iter().any(|item| {
        matches!(item, FeedItem::AssistantMessage { text, .. } if !text.contains("Let me think"))
    }), "vm.message_list should show assistant without thinking text");
}

// ─── Test 5: Error Handling E2E ──────────────────────────────────────────────

/// Process: Error event
#[test]
fn test_error_handling_e2e() {
    let mut state = make_test_state();

    handle_agent_event(&mut state, AgentEvent::Error {
        message: "API rate limit exceeded".to_string(),
        error_type: "rate_limit".to_string(),
        recoverable: true,
        context: "When calling the LLM API".to_string(),
    });

    // Assert state
    assert!(!state.agent_running, "agent_running should be false after error");
    assert!(state.messages.iter().any(|m| matches!(m, MessageItem::Error { .. })),
        "state.messages should have Error item");

    // Build viewmodels and assert
    let vm = build_viewmodels(&state);

    // Error appears as SystemNotice in feed
    assert!(vm.message_list.feed.items().iter().any(|item| {
        matches!(item, FeedItem::SystemNotice { text } if text.contains("rate limit") || text.contains("Error"))
    }), "vm.message_list should show error");
}

// ─── Test 6: Token Usage Tracking E2E ────────────────────────────────────────

/// Process: MessageStart → MessageUpdate → MessageEnd → TokenUsage → AgentEnd
#[test]
fn test_token_usage_tracking_e2e() {
    let mut state = make_test_state();

    handle_agent_event(&mut state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::MessageUpdate {
        message: agent_message("assistant", "Response text"),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::MessageEnd {
        message: agent_message("assistant", "Response text"),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::TokenUsage {
        prompt_tokens: 100,
        completion_tokens: 50,
        total_tokens: 150,
        context_window: 128_000,
    });
    handle_agent_event(&mut state, AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: runie_agent::TokenUsage::default(),
    });

    // Assert state session_token_usage
    assert_eq!(state.session_token_usage.prompt_tokens, 100);
    assert_eq!(state.session_token_usage.completion_tokens, 50);
    assert_eq!(state.session_token_usage.total_tokens, 150);

    // Build viewmodels and assert
    let vm = build_viewmodels(&state);

    assert_eq!(vm.status_bar.session_token_usage.total_tokens, 150,
        "vm.status_bar should show correct total tokens");
    assert_eq!(vm.status_bar.session_token_usage.prompt_tokens, 100,
        "vm.status_bar should show correct prompt tokens");
    assert_eq!(vm.status_bar.session_token_usage.completion_tokens, 50,
        "vm.status_bar should show correct completion tokens");
}

// ─── Test 7: Multi-Turn E2E ──────────────────────────────────────────────────

/// Process: Full first turn → TurnEnd → Second MessageStart → MessageUpdate →
/// MessageEnd → AgentEnd
#[test]
fn test_multi_turn_e2e() {
    let mut state = make_test_state();

    // First turn
    handle_agent_event(&mut state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::MessageUpdate {
        message: agent_message("assistant", "First response"),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::MessageEnd {
        message: agent_message("assistant", "First response"),
        turn: 0,
    });
    // Set agent_start_time before TurnEnd so separator is added
    state.agent_start_time = Some(std::time::Instant::now());
    std::thread::sleep(std::time::Duration::from_millis(10)); // Ensure some elapsed time
    handle_agent_event(&mut state, AgentEvent::TurnEnd {
        turn: 0,
        message_count: 2,
        tool_results_count: 0,
        token_usage: runie_agent::TokenUsage {
            input: 50,
            output: 25,
            cache_read: 0,
            cache_write: 0,
            total_tokens: 75,
        },
    });

    // Second turn
    handle_agent_event(&mut state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });
    handle_agent_event(&mut state, AgentEvent::MessageUpdate {
        message: agent_message("assistant", "Second response"),
        turn: 1,
    });
    handle_agent_event(&mut state, AgentEvent::MessageEnd {
        message: agent_message("assistant", "Second response"),
        turn: 1,
    });
    // Set agent_start_time before TurnEnd for turn 1
    state.agent_start_time = Some(std::time::Instant::now());
    std::thread::sleep(std::time::Duration::from_millis(10));
    handle_agent_event(&mut state, AgentEvent::TurnEnd {
        turn: 1,
        message_count: 2,
        tool_results_count: 0,
        token_usage: runie_agent::TokenUsage {
            input: 50,
            output: 25,
            cache_read: 0,
            cache_write: 0,
            total_tokens: 75,
        },
    });
    handle_agent_event(&mut state, AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 2,
        final_token_usage: runie_agent::TokenUsage::default(),
    });

    // Assert messages array has both responses
    let assistant_count = state.messages.iter()
        .filter(|m| matches!(m, MessageItem::Assistant { .. }))
        .count();
    assert_eq!(assistant_count, 2, "Should have 2 Assistant messages (one per turn)");

    // Assert turn info stored in state fields (not as Separator in messages)
    assert!(state.last_turn_duration_secs.is_some(), 
        "last_turn_duration_secs should be set after TurnEnd");
    assert!(state.last_turn_tokens.is_some(), 
        "last_turn_tokens should be set after TurnEnd");

    // Build viewmodels and assert
    let vm = build_viewmodels(&state);

    // Should have 2 assistant messages in feed
    let assistant_count_in_feed = vm.message_list.feed.items().iter()
        .filter(|item| matches!(item, FeedItem::AssistantMessage { .. }))
        .count();
    assert_eq!(assistant_count_in_feed, 2, "vm.message_list should show both turns");
}

// ─── Test 8: Permission Request E2E ──────────────────────────────────────────

/// Process: PermissionRequest event
#[test]
fn test_permission_request_e2e() {
    let mut state = make_test_state();

    handle_agent_event(&mut state, AgentEvent::PermissionRequest {
        tool_call_id: "call_perm_123".to_string(),
        tool_name: "bash".to_string(),
        tool_args: r#"{"command": "rm -rf /"}"#.to_string(),
        tool_description: "Execute a bash command".to_string(),
        turn: 0,
        context_window_usage: 0.25,
    });

    // Assert state.mode is Permission
    assert_eq!(state.mode, crate::tui::state::TuiMode::Permission,
        "state.mode should be Permission");

    // Assert permission_modal has tool info
    assert_eq!(state.permission_modal.tool.as_deref(), Some("bash"),
        "permission_modal.tool should be set");
    assert_eq!(state.permission_modal.tool_call_id.as_deref(), Some("call_perm_123"),
        "permission_modal.tool_call_id should be set");
    assert!(state.permission_modal.args.as_ref().map_or(false, |a| a.contains("rm -rf")),
        "permission_modal.args should contain the command");

    // Build viewmodels and assert
    let vm = build_viewmodels(&state);

    assert!(vm.permission_modal.is_some(), "vm.permission_modal should be Some");
    if let Some(ref perm_vm) = vm.permission_modal {
        assert_eq!(perm_vm.tool, "bash", "vm.permission_modal.tool should be bash");
        assert!(perm_vm.visible, "vm.permission_modal.visible should be true");
    }
}

// ─── Additional Integration Tests ────────────────────────────────────────────

/// Verify agent_running state transitions correctly
#[test]
fn test_agent_running_state_transitions() {
    let mut state = make_test_state();

    assert!(!state.agent_running, "Agent should not be running initially");

    handle_agent_event(&mut state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 0,
    });
    assert!(state.agent_running, "Agent should be running after MessageStart");

    let vm_during = build_viewmodels(&state);
    assert!(vm_during.agent_list.agent_running, "ViewModel should show agent running during execution");

    handle_agent_event(&mut state, AgentEvent::MessageEnd {
        message: agent_message("assistant", "Done"),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: runie_agent::TokenUsage::default(),
    });

    assert!(!state.agent_running, "Agent should not be running after AgentEnd");

    let vm_after = build_viewmodels(&state);
    assert!(!vm_after.agent_list.agent_running, "ViewModel should show agent not running after end");
}

/// Verify status_bar model persistence across agent runs
#[test]
fn test_model_persists_across_agent_runs() {
    let mut state = make_test_state();

    // First agent run
    handle_agent_event(&mut state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::MessageEnd {
        message: agent_message("assistant", "First run"),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: runie_agent::TokenUsage::default(),
    });

    assert_eq!(state.current_model.as_deref(), Some("MiniMax-M2.7-highspeed"),
        "current_model should persist after first run");

    // Second agent run
    handle_agent_event(&mut state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::MessageEnd {
        message: agent_message("assistant", "Second run"),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: runie_agent::TokenUsage::default(),
    });

    assert_eq!(state.current_model.as_deref(), Some("MiniMax-M2.7-highspeed"),
        "current_model should persist after second run");

    let vm = build_viewmodels(&state);
    assert_eq!(vm.status_bar.current_model.as_deref(), Some("MiniMax-M2.7-highspeed"),
        "ViewModel should preserve model across runs");
}

/// Verify thinking state cleanup on agent end
#[test]
fn test_thinking_state_cleanup() {
    let mut state = make_test_state();

    handle_agent_event(&mut state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::ThinkingUpdate {
        text: "Thinking...".to_string(),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::MessageEnd {
        message: agent_message("assistant", "Done"),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: runie_agent::TokenUsage::default(),
    });

    assert!(state.thinking.is_none(), "thinking should be None after AgentEnd");
}

/// Verify multiple tool calls in sequence
#[test]
fn test_multiple_tool_calls_sequence() {
    let mut state = make_test_state();

    handle_agent_event(&mut state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 0,
    });

    // First tool
    handle_agent_event(&mut state, AgentEvent::ToolExecutionStart {
        tool_call_id: "call_1".to_string(),
        tool_name: "read".to_string(),
        tool_args: r#"{"path": "file1.txt"}"#.to_string(),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::ToolExecutionEnd {
        tool_call_id: "call_1".to_string(),
        tool_name: "read".to_string(),
        tool_args: r#"{"path": "file1.txt"}"#.to_string(),
        result: ToolResult {
            tool_call_id: "call_1".to_string(),
            tool_name: "read".to_string(),
            input: serde_json::json!({"path": "file1.txt"}),
            content: vec![ContentPart::Text { text: "content1".to_string() }],
            is_error: false,
        },
        duration_ms: 50,
        turn: 0,
    });

    // Second tool
    handle_agent_event(&mut state, AgentEvent::ToolExecutionStart {
        tool_call_id: "call_2".to_string(),
        tool_name: "write".to_string(),
        tool_args: r#"{"path": "file2.txt"}"#.to_string(),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::ToolExecutionEnd {
        tool_call_id: "call_2".to_string(),
        tool_name: "write".to_string(),
        tool_args: r#"{"path": "file2.txt"}"#.to_string(),
        result: ToolResult {
            tool_call_id: "call_2".to_string(),
            tool_name: "write".to_string(),
            input: serde_json::json!({"path": "file2.txt"}),
            content: vec![ContentPart::Text { text: "content2".to_string() }],
            is_error: false,
        },
        duration_ms: 75,
        turn: 0,
    });

    handle_agent_event(&mut state, AgentEvent::MessageEnd {
        message: agent_message("assistant", "Done with both tools"),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: runie_agent::TokenUsage::default(),
    });

    // Count tool calls
    let tool_call_count = state.messages.iter()
        .filter(|m| matches!(m, MessageItem::ToolCall { .. }))
        .count();
    assert_eq!(tool_call_count, 2, "Should have 2 tool calls");

    // Verify both tools have results
    let tools_with_results = state.messages.iter()
        .filter(|m| {
            if let MessageItem::ToolCall { result, .. } = m {
                result.is_some()
            } else {
                false
            }
        })
        .count();
    assert_eq!(tools_with_results, 2, "Both tool calls should have results");
}

/// Verify tool error handling
#[test]
fn test_tool_error_handling() {
    let mut state = make_test_state();

    handle_agent_event(&mut state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::ToolExecutionStart {
        tool_call_id: "call_error".to_string(),
        tool_name: "bash".to_string(),
        tool_args: r#"{"command": "exit 1"}"#.to_string(),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::ToolExecutionEnd {
        tool_call_id: "call_error".to_string(),
        tool_name: "bash".to_string(),
        tool_args: r#"{"command": "exit 1"}"#.to_string(),
        result: ToolResult {
            tool_call_id: "call_error".to_string(),
            tool_name: "bash".to_string(),
            input: serde_json::json!({"command": "exit 1"}),
            content: vec![ContentPart::Text { text: "Command failed with exit code 1".to_string() }],
            is_error: true,
        },
        duration_ms: 100,
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::MessageEnd {
        message: agent_message("assistant", "The command failed."),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: runie_agent::TokenUsage::default(),
    });

    // Find the tool call and verify is_error flag
    let has_error_tool = state.messages.iter().any(|m| {
        if let MessageItem::ToolCall { is_error, .. } = m {
            *is_error
        } else {
            false
        }
    });
    assert!(has_error_tool, "Should have a tool call with is_error=true");
}

/// Verify global_tags shows running state correctly
#[test]
fn test_global_tags_running_state() {
    let mut state = make_test_state();

    let vm_idle = build_viewmodels(&state);
    // At idle, global_tags.left is None
    assert!(vm_idle.global_tags.left.is_none(),
        "GlobalTagsViewModel should show idle state initially");

    handle_agent_event(&mut state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 0,
    });

    let vm_running = build_viewmodels(&state);
    assert!(vm_running.global_tags.left.is_some(),
        "GlobalTagsViewModel should show running state during agent execution");
}

/// Verify cumulative token usage across multiple TokenUsage events
#[test]
fn test_cumulative_token_usage() {
    let mut state = make_test_state();

    handle_agent_event(&mut state, AgentEvent::TokenUsage {
        prompt_tokens: 100,
        completion_tokens: 50,
        total_tokens: 150,
        context_window: 128_000,
    });
    handle_agent_event(&mut state, AgentEvent::TokenUsage {
        prompt_tokens: 200,
        completion_tokens: 100,
        total_tokens: 300,
        context_window: 128_000,
    });

    assert_eq!(state.session_token_usage.prompt_tokens, 300);
    assert_eq!(state.session_token_usage.completion_tokens, 150);
    assert_eq!(state.session_token_usage.total_tokens, 450);

    let vm = build_viewmodels(&state);
    assert_eq!(vm.status_bar.session_token_usage.total_tokens, 450);
}
