//! ReplyProvider tests.
//!
//! Comprehensive tests verifying ReplyProvider event emission and TUI integration.
//! Tests all 7 scenarios: simple, tool, streaming, streaming tool, error, context, long reasoning.

use crate::components::message_list::feed::FeedItem;
use crate::components::MessageItem;
use crate::tui::state::AppState;
use crate::tui::update::agent::handle_agent_event;
use crate::tui::view_models::ViewModels;
use crate::components::message_list::render::WrapCache;
use crate::components::CommandPalette;
use runie_agent::{AgentEvent, AgentMessage, ContentPart, ToolResult};

/// Helper: Create an AgentMessage with given role and content text.
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

/// Helper: Create AppState ready for testing.
fn make_test_state() -> AppState {
    let mut state = AppState::default();
    state.current_model = Some("MiniMax-M2.7-highspeed".to_string());
    state
}

/// Helper: Build viewmodels from app state.
fn build_viewmodels(state: &AppState) -> ViewModels {
    let palette = CommandPalette::default();
    let wrap_cache = WrapCache::default();
    ViewModels::from_app_state(state, &palette, wrap_cache)
}

/// Helper: Simulate a complete simple message flow.
fn simulate_simple_flow(state: &mut AppState) {
    handle_agent_event(state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 0,
    });
    handle_agent_event(state, AgentEvent::ThinkingUpdate {
        text: "The user wants me to say hello and tell them the current time.".to_string(),
        turn: 0,
    });
    handle_agent_event(state, AgentEvent::MessageUpdate {
        message: agent_message("assistant", "Hello! I'm here and ready to help!"),
        turn: 0,
    });
    handle_agent_event(state, AgentEvent::MessageEnd {
        message: agent_message("assistant", "Hello! I'm here and ready to help!"),
        turn: 0,
    });
    handle_agent_event(state, AgentEvent::TokenUsage {
        prompt_tokens: 29,
        completion_tokens: 101,
        total_tokens: 130,
        context_window: 128_000,
    });
    handle_agent_event(state, AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: runie_agent::TokenUsage::default(),
    });
}

/// Helper: Simulate a tool call flow (calculator).
fn simulate_tool_flow(state: &mut AppState) {
    handle_agent_event(state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 0,
    });
    handle_agent_event(state, AgentEvent::ThinkingUpdate {
        text: "The user is asking me to calculate 15 + 27.".to_string(),
        turn: 0,
    });
    handle_agent_event(state, AgentEvent::ToolExecutionStart {
        tool_call_id: "call_function_m5jy6idn55ce_1".to_string(),
        tool_name: "calculator".to_string(),
        tool_args: "{\"expression\": \"15 + 27\"}".to_string(),
        turn: 0,
    });
    handle_agent_event(state, AgentEvent::ToolExecutionEnd {
        tool_call_id: "call_function_m5jy6idn55ce_1".to_string(),
        tool_name: "calculator".to_string(),
        tool_args: "{\"expression\": \"15 + 27\"}".to_string(),
        result: ToolResult {
            tool_call_id: "call_function_m5jy6idn55ce_1".to_string(),
            tool_name: "calculator".to_string(),
            input: serde_json::json!({"expression": "15 + 27"}),
            content: vec![ContentPart::Text { text: "42".to_string() }],
            is_error: false,
        },
        duration_ms: 100,
        turn: 0,
    });
    handle_agent_event(state, AgentEvent::MessageUpdate {
        message: AgentMessage {
            role: "assistant".to_string(),
            content: vec![ContentPart::Text { text: "The result is 42.".to_string() }],
            timestamp: 0,
            usage: None,
            stop_reason: None,
            error_message: None,
            tool_calls: vec![],
        },
        turn: 0,
    });
    handle_agent_event(state, AgentEvent::MessageEnd {
        message: agent_message("assistant", "The result is 42."),
        turn: 0,
    });
    handle_agent_event(state, AgentEvent::TokenUsage {
        prompt_tokens: 193,
        completion_tokens: 59,
        total_tokens: 252,
        context_window: 128_000,
    });
    handle_agent_event(state, AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: runie_agent::TokenUsage::default(),
    });
}

/// Helper: Simulate a streaming message flow (count 1-3).
fn simulate_streaming_flow(state: &mut AppState) {
    handle_agent_event(state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 0,
    });
    handle_agent_event(state, AgentEvent::ThinkingUpdate {
        text: "The user".to_string(),
        turn: 0,
    });
    handle_agent_event(state, AgentEvent::ThinkingUpdate {
        text: " wants me to count from 1 to 3.".to_string(),
        turn: 0,
    });
    handle_agent_event(state, AgentEvent::MessageUpdate {
        message: agent_message("assistant", "1"),
        turn: 0,
    });
    handle_agent_event(state, AgentEvent::MessageUpdate {
        message: agent_message("assistant", ", 2"),
        turn: 0,
    });
    handle_agent_event(state, AgentEvent::MessageUpdate {
        message: agent_message("assistant", ", 3"),
        turn: 0,
    });
    handle_agent_event(state, AgentEvent::MessageEnd {
        message: agent_message("assistant", "1, 2, 3"),
        turn: 0,
    });
    handle_agent_event(state, AgentEvent::TokenUsage {
        prompt_tokens: 27,
        completion_tokens: 24,
        total_tokens: 51,
        context_window: 128_000,
    });
    handle_agent_event(state, AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: runie_agent::TokenUsage::default(),
    });
}

/// Helper: Simulate a streaming tool call flow (bash ls).
fn simulate_stream_tool_flow(state: &mut AppState) {
    handle_agent_event(state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 0,
    });
    handle_agent_event(state, AgentEvent::ThinkingUpdate {
        text: "The user wants me to list files in the current directory using the bash tool.".to_string(),
        turn: 0,
    });
    handle_agent_event(state, AgentEvent::ToolExecutionStart {
        tool_call_id: "call_function_tliic6kofgz2_1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "{\"command\": \"ls -la\"}".to_string(),
        turn: 0,
    });
    handle_agent_event(state, AgentEvent::ToolExecutionEnd {
        tool_call_id: "call_function_tliic6kofgz2_1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "{\"command\": \"ls -la\"}".to_string(),
        result: ToolResult {
            tool_call_id: "call_function_tliic6kofgz2_1".to_string(),
            tool_name: "bash".to_string(),
            input: serde_json::json!({"command": "ls -la"}),
            content: vec![ContentPart::Text { text: "total 32\n-rw-r--r--  1 admin  staff  128 May 31 18:24 ..".to_string() }],
            is_error: false,
        },
        duration_ms: 50,
        turn: 0,
    });
    handle_agent_event(state, AgentEvent::MessageEnd {
        message: agent_message("assistant", "Here's the file listing..."),
        turn: 0,
    });
    handle_agent_event(state, AgentEvent::TokenUsage {
        prompt_tokens: 174,
        completion_tokens: 44,
        total_tokens: 218,
        context_window: 128_000,
    });
    handle_agent_event(state, AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: runie_agent::TokenUsage::default(),
    });
}

/// Helper: Simulate an error response.
fn simulate_error_flow(state: &mut AppState) {
    handle_agent_event(state, AgentEvent::Error {
        message: "MiniMax API error: status_code=2013".to_string(),
        error_type: "api_error".to_string(),
        recoverable: false,
        context: "invalid model 'invalid-model-name'".to_string(),
    });
}

/// Helper: Simulate a context/multi-turn response (Alice).
fn simulate_context_flow(state: &mut AppState) {
    handle_agent_event(state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 0,
    });
    handle_agent_event(state, AgentEvent::ThinkingUpdate {
        text: "The user".to_string(),
        turn: 0,
    });
    handle_agent_event(state, AgentEvent::ThinkingUpdate {
        text: " just told me their name is Alice, so I should answer that.".to_string(),
        turn: 0,
    });
    handle_agent_event(state, AgentEvent::MessageUpdate {
        message: agent_message("assistant", "Your name is Alice!"),
        turn: 0,
    });
    handle_agent_event(state, AgentEvent::MessageEnd {
        message: agent_message("assistant", "Your name is Alice!"),
        turn: 0,
    });
    handle_agent_event(state, AgentEvent::TokenUsage {
        prompt_tokens: 47,
        completion_tokens: 25,
        total_tokens: 72,
        context_window: 128_000,
    });
    handle_agent_event(state, AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: runie_agent::TokenUsage::default(),
    });
}

/// Helper: Simulate a long reasoning response (peanut butter sandwich).
fn simulate_long_reasoning_flow(state: &mut AppState) {
    handle_agent_event(state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 0,
    });
    handle_agent_event(state, AgentEvent::ThinkingStart { turn: 0 });
    handle_agent_event(state, AgentEvent::ThinkingUpdate {
        text: "The user wants step-by-step instructions for making a peanut butter sandwich.".to_string(),
        turn: 0,
    });
    // Simulate enough time for thought indicator
    std::thread::sleep(std::time::Duration::from_millis(600));
    handle_agent_event(state, AgentEvent::ThinkingEnd {
        duration_ms: 600,
        turn: 0,
    });
    handle_agent_event(state, AgentEvent::MessageUpdate {
        message: agent_message("assistant", "# How to Make a Peanut Butter Sandwich\n\n**Ingredients:**\n- 2 slices of bread\n- Peanut butter\n- Optional: jelly, honey, or banana\n\n"),
        turn: 0,
    });
    handle_agent_event(state, AgentEvent::MessageEnd {
        message: agent_message("assistant", "# How to Make a Peanut Butter Sandwich..."),
        turn: 0,
    });
    handle_agent_event(state, AgentEvent::TokenUsage {
        prompt_tokens: 47,
        completion_tokens: 1200,
        total_tokens: 1247,
        context_window: 128_000,
    });
    handle_agent_event(state, AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: runie_agent::TokenUsage::default(),
    });
}

// ─── Test 1: Simple Response ──────────────────────────────────────────────────

#[test]
fn test_simple_message_flow_assistant_message() {
    let mut state = make_test_state();
    simulate_simple_flow(&mut state);

    assert!(state.messages.iter().any(|m| matches!(
        m,
        MessageItem::Assistant { text, .. } if text.contains("Hello")
    )), "Should have assistant message with Hello");
}

#[test]
fn test_simple_message_flow_viewmodel() {
    let mut state = make_test_state();
    simulate_simple_flow(&mut state);
    let vm = build_viewmodels(&state);

    assert!(vm.message_list.feed.items().iter().any(|item| {
        matches!(item, FeedItem::AssistantMessage { text, .. } if text.contains("Hello"))
    }), "ViewModel should have assistant message with Hello");
}

#[test]
fn test_simple_message_flow_token_usage() {
    let mut state = make_test_state();
    simulate_simple_flow(&mut state);

    assert_eq!(state.session_token_usage.total_tokens, 130);
    assert_eq!(state.session_token_usage.prompt_tokens, 29);
    assert_eq!(state.session_token_usage.completion_tokens, 101);
}

#[test]
fn test_simple_message_flow_has_assistant() {
    let mut state = make_test_state();
    simulate_simple_flow(&mut state);

    assert!(state.messages.iter().any(|m| matches!(m, MessageItem::Assistant { .. })),
        "Should have Assistant message");
}

// ─── Test 2: Tool Call Response ────────────────────────────────────────────────

#[test]
fn test_tool_call_flow_creates_toolcall_item() {
    let mut state = make_test_state();
    simulate_tool_flow(&mut state);

    assert!(state.messages.iter().any(|m| matches!(
        m,
        MessageItem::ToolCall { name, .. } if name.contains("call_function_m5jy6idn55ce_1")
    )), "Should have ToolCall item with call_id");
}

#[test]
fn test_tool_call_flow_tool_result() {
    let mut state = make_test_state();
    simulate_tool_flow(&mut state);

    let has_tool_result = state.messages.iter().any(|m| {
        if let MessageItem::ToolCall { result, .. } = m {
            result.as_ref().map_or(false, |r| r.contains("42"))
        } else {
            false
        }
    });
    assert!(has_tool_result, "ToolCall should have result containing 42");
}

#[test]
fn test_tool_call_flow_token_usage() {
    let mut state = make_test_state();
    simulate_tool_flow(&mut state);

    assert_eq!(state.session_token_usage.total_tokens, 252);
    assert_eq!(state.session_token_usage.prompt_tokens, 193);
    assert_eq!(state.session_token_usage.completion_tokens, 59);
}

// ─── Test 3: Streaming Response ───────────────────────────────────────────────

#[test]
fn test_streaming_flow_has_assistant_message() {
    let mut state = make_test_state();
    simulate_streaming_flow(&mut state);

    let has_assistant = state.messages.iter().any(|m| {
        matches!(m, MessageItem::Assistant { .. })
    });

    assert!(has_assistant, "Should have assistant message after streaming");
}

#[test]
fn test_streaming_flow_viewmodel() {
    let mut state = make_test_state();
    simulate_streaming_flow(&mut state);
    let vm = build_viewmodels(&state);

    assert!(vm.message_list.feed.items().iter().any(|item| {
        matches!(item, FeedItem::AssistantMessage { .. })
    }), "ViewModel should have assistant message after streaming");
}

#[test]
fn test_streaming_flow_token_usage() {
    let mut state = make_test_state();
    simulate_streaming_flow(&mut state);

    assert_eq!(state.session_token_usage.total_tokens, 51);
    assert_eq!(state.session_token_usage.prompt_tokens, 27);
    assert_eq!(state.session_token_usage.completion_tokens, 24);
}

// ─── Test 4: Streaming Tool Response ──────────────────────────────────────────

#[test]
fn test_stream_tool_flow_has_tool_call() {
    let mut state = make_test_state();
    simulate_stream_tool_flow(&mut state);

    assert!(state.messages.iter().any(|m| matches!(
        m,
        MessageItem::ToolCall { name, .. } if name.contains("bash") || name.contains("call_function")
    )), "Should have bash tool call");
}

#[test]
fn test_stream_tool_flow_token_usage() {
    let mut state = make_test_state();
    simulate_stream_tool_flow(&mut state);

    assert_eq!(state.session_token_usage.total_tokens, 218);
    assert_eq!(state.session_token_usage.prompt_tokens, 174);
    assert_eq!(state.session_token_usage.completion_tokens, 44);
}

#[test]
fn test_stream_tool_flow_viewmodel() {
    let mut state = make_test_state();
    simulate_stream_tool_flow(&mut state);

    assert!(state.messages.iter().any(|m| matches!(m, MessageItem::ToolCall { .. })),
        "Should have ToolCall in state.messages");
}

// ─── Test 5: Error Response ───────────────────────────────────────────────────

#[test]
fn test_error_response_error_event() {
    let mut state = make_test_state();
    simulate_error_flow(&mut state);

    assert!(state.messages.iter().any(|m| {
        if let MessageItem::Error { message, .. } = m {
            message.contains("2013") || message.contains("invalid") || message.contains("error")
        } else {
            false
        }
    }), "Should have Error message");
}

#[test]
fn test_error_response_agent_not_running() {
    let mut state = make_test_state();
    simulate_error_flow(&mut state);

    assert!(!state.agent_running, "Agent should not be running after error");
}

#[test]
fn test_error_response_no_message_content() {
    let mut state = make_test_state();
    simulate_error_flow(&mut state);

    let has_error = state.messages.iter().any(|m| matches!(m, MessageItem::Error { .. }));
    let has_assistant = state.messages.iter().any(|m| matches!(m, MessageItem::Assistant { .. }));

    assert!(has_error, "Should have Error message");
    assert!(!has_assistant, "Should NOT have Assistant message for error response");
}

// ─── Test 6: Context/Multi-turn Response ─────────────────────────────────────

#[test]
fn test_context_response_assistant_message() {
    let mut state = make_test_state();
    simulate_context_flow(&mut state);

    assert!(state.messages.iter().any(|m| matches!(
        m,
        MessageItem::Assistant { text, .. } if text.contains("Alice")
    )), "Should have assistant message with Alice");
}

#[test]
fn test_context_response_token_usage() {
    let mut state = make_test_state();
    simulate_context_flow(&mut state);

    assert_eq!(state.session_token_usage.total_tokens, 72);
    assert_eq!(state.session_token_usage.prompt_tokens, 47);
    assert_eq!(state.session_token_usage.completion_tokens, 25);
}

#[test]
fn test_context_response_viewmodel() {
    let mut state = make_test_state();
    simulate_context_flow(&mut state);
    let vm = build_viewmodels(&state);

    assert!(vm.message_list.feed.items().iter().any(|item| {
        matches!(item, FeedItem::AssistantMessage { text, .. } if text.contains("Alice"))
    }), "ViewModel should have assistant message with Alice");
}

// ─── Test 7: Long Reasoning Response ──────────────────────────────────────────

#[test]
fn test_long_reasoning_has_assistant() {
    let mut state = make_test_state();
    simulate_long_reasoning_flow(&mut state);

    assert!(state.messages.iter().any(|m| matches!(m, MessageItem::Assistant { .. })),
        "Should have Assistant message for long reasoning");
}

#[test]
fn test_long_reasoning_token_usage() {
    let mut state = make_test_state();
    simulate_long_reasoning_flow(&mut state);

    assert!(state.session_token_usage.total_tokens > 1000,
        "Long reasoning should have > 1000 tokens, got: {}", state.session_token_usage.total_tokens);
}

#[test]
fn test_long_reasoning_viewmodel() {
    let mut state = make_test_state();
    simulate_long_reasoning_flow(&mut state);
    let vm = build_viewmodels(&state);

    assert!(vm.message_list.feed.items().iter().any(|item| {
        matches!(item, FeedItem::AssistantMessage { .. })
    }), "ViewModel should have assistant message");
}

#[test]
fn test_long_reasoning_thinking() {
    let mut state = make_test_state();
    simulate_long_reasoning_flow(&mut state);

    assert!(state.messages.iter().any(|m| matches!(m, MessageItem::Thought { .. })),
        "Should have Thought item for long reasoning");
}

// ─── Test 8: ViewModel Updates ───────────────────────────────────────────────

#[test]
fn test_viewmodel_message_list_builds_correctly() {
    let mut state = make_test_state();

    handle_agent_event(&mut state, AgentEvent::Message {
        role: "user".to_string(),
        content: "Hello!".to_string(),
    });

    simulate_simple_flow(&mut state);
    let vm = build_viewmodels(&state);

    let items = vm.message_list.feed.items();
    assert!(items.iter().any(|i| matches!(i, FeedItem::UserMessage { .. })),
        "Should have User message in viewmodel");
    assert!(items.iter().any(|i| matches!(i, FeedItem::AssistantMessage { .. })),
        "Should have Assistant message in viewmodel");
}

#[test]
fn test_viewmodel_after_tool_flow() {
    let mut state = make_test_state();

    handle_agent_event(&mut state, AgentEvent::Message {
        role: "user".to_string(),
        content: "Calculate 15 + 27".to_string(),
    });

    simulate_tool_flow(&mut state);
    let vm = build_viewmodels(&state);

    assert!(vm.message_list.feed.items().iter().any(|i| matches!(i, FeedItem::UserMessage { .. })),
        "Should have User message in feed");
    assert!(state.messages.iter().any(|m| matches!(m, MessageItem::ToolCall { .. })),
        "Should have ToolCall in state.messages");
}

#[test]
fn test_viewmodel_agent_running_state() {
    let mut state = make_test_state();

    assert!(!state.agent_running, "Agent should not be running initially");

    handle_agent_event(&mut state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 0,
    });

    assert!(state.agent_running, "Agent should be running after MessageStart");

    let vm_before_end = build_viewmodels(&state);
    assert!(vm_before_end.agent_list.agent_running, "ViewModel should show agent running");

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
}

// ─── Scenario Routing Tests ──────────────────────────────────────────────────

#[test]
fn test_scenario_routing_simple() {
    let mut state = make_test_state();
    simulate_simple_flow(&mut state);

    assert!(state.messages.iter().any(|m| matches!(m, MessageItem::Assistant { .. })),
        "hello should produce assistant message");
}

#[test]
fn test_scenario_routing_tool() {
    let mut state = make_test_state();
    simulate_tool_flow(&mut state);

    assert!(state.messages.iter().any(|m| matches!(m, MessageItem::ToolCall { .. })),
        "tool should produce tool call");
}

#[test]
fn test_scenario_routing_stream() {
    let mut state = make_test_state();
    simulate_streaming_flow(&mut state);

    assert!(state.messages.iter().any(|m| {
        matches!(m, MessageItem::Assistant { .. })
    }), "count should produce assistant message");
}

#[test]
fn test_scenario_routing_stream_tool() {
    let mut state = make_test_state();
    simulate_stream_tool_flow(&mut state);

    assert!(state.messages.iter().any(|m| matches!(m, MessageItem::ToolCall { .. })),
        "bash should produce tool call");
}

#[test]
fn test_scenario_routing_error() {
    let mut state = make_test_state();
    simulate_error_flow(&mut state);

    assert!(state.messages.iter().any(|m| matches!(m, MessageItem::Error { .. })),
        "error should produce error message");
}

#[test]
fn test_scenario_routing_context() {
    let mut state = make_test_state();
    simulate_context_flow(&mut state);

    assert!(state.messages.iter().any(|m| {
        if let MessageItem::Assistant { text, .. } = m {
            text.contains("Alice")
        } else {
            false
        }
    }), "context should produce Alice message");
}

#[test]
fn test_scenario_routing_long() {
    let mut state = make_test_state();
    simulate_long_reasoning_flow(&mut state);

    assert!(state.messages.iter().any(|m| matches!(m, MessageItem::Assistant { .. })),
        "long should produce assistant message");
}

// ─── Edge Case Tests ─────────────────────────────────────────────────────────

#[test]
fn test_multiple_tool_calls() {
    let mut state = make_test_state();

    handle_agent_event(&mut state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 0,
    });

    handle_agent_event(&mut state, AgentEvent::ToolExecutionStart {
        tool_call_id: "call_1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "{}".to_string(),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::ToolExecutionEnd {
        tool_call_id: "call_1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "{}".to_string(),
        result: ToolResult {
            tool_call_id: "call_1".to_string(),
            tool_name: "bash".to_string(),
            input: serde_json::json!({}),
            content: vec![ContentPart::Text { text: "result1".to_string() }],
            is_error: false,
        },
        duration_ms: 50,
        turn: 0,
    });

    handle_agent_event(&mut state, AgentEvent::ToolExecutionStart {
        tool_call_id: "call_2".to_string(),
        tool_name: "read".to_string(),
        tool_args: "{}".to_string(),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::ToolExecutionEnd {
        tool_call_id: "call_2".to_string(),
        tool_name: "read".to_string(),
        tool_args: "{}".to_string(),
        result: ToolResult {
            tool_call_id: "call_2".to_string(),
            tool_name: "read".to_string(),
            input: serde_json::json!({}),
            content: vec![ContentPart::Text { text: "result2".to_string() }],
            is_error: false,
        },
        duration_ms: 50,
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

    let tool_calls: Vec<_> = state.messages.iter()
        .filter_map(|m| {
            if let MessageItem::ToolCall { name, .. } = m {
                Some(name.clone())
            } else {
                None
            }
        })
        .collect();

    assert_eq!(tool_calls.len(), 2, "Should have 2 tool calls");
    assert!(tool_calls.contains(&"call_1".to_string()));
    assert!(tool_calls.contains(&"call_2".to_string()));
}

#[test]
fn test_error_tool_call() {
    let mut state = make_test_state();

    handle_agent_event(&mut state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 0,
    });

    handle_agent_event(&mut state, AgentEvent::ToolExecutionStart {
        tool_call_id: "call_fail".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "{}".to_string(),
        turn: 0,
    });

    handle_agent_event(&mut state, AgentEvent::ToolExecutionEnd {
        tool_call_id: "call_fail".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "{}".to_string(),
        result: ToolResult {
            tool_call_id: "call_fail".to_string(),
            tool_name: "bash".to_string(),
            input: serde_json::json!({}),
            content: vec![ContentPart::Text { text: "Error: command not found".to_string() }],
            is_error: true,
        },
        duration_ms: 50,
        turn: 0,
    });

    handle_agent_event(&mut state, AgentEvent::MessageEnd {
        message: agent_message("assistant", "There was an error"),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: runie_agent::TokenUsage::default(),
    });

    let has_error_tool = state.messages.iter().any(|m| {
        if let MessageItem::ToolCall { is_error, .. } = m {
            *is_error
        } else {
            false
        }
    });

    assert!(has_error_tool, "Should have an error tool call");
}
