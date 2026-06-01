//! Full-cycle integration tests for reply provider.
//!
//! Tests complete flow: user input → TUI update → agent events → viewmodels.
//! Verifies state consistency across the entire request-response cycle.

use crate::components::message_list::feed::FeedItem;
use crate::components::message_list::render::WrapCache;
use crate::components::CommandPalette;
use crate::components::MessageItem;
use crate::tui::state::{AppState, Cmd, Msg, TuiMode};
use crate::tui::update::agent::handle_agent_event;
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

/// Create AppState ready for testing with a model configured.
fn make_test_state() -> AppState {
    let mut state = AppState::default();
    state.current_model = Some("minimax".to_string());
    state
}

/// Build viewmodels from app state.
fn build_viewmodels(state: &AppState) -> ViewModels {
    let palette = CommandPalette::default();
    let wrap_cache = WrapCache::default();
    ViewModels::from_app_state(state, &palette, wrap_cache)
}

/// Helper: Submit a message via update and return commands.
fn submit_message(state: &mut AppState, palette: &mut CommandPalette, text: &str) -> Vec<Cmd> {
    state.textarea.insert_str(text);
    crate::tui::update::update(state, palette, Msg::Submit)
}

// ─── Test 1: Full Cycle Simple Message ───────────────────────────────────────

#[test]
fn test_full_cycle_simple_message() {
    let mut state = make_test_state();
    let mut palette = CommandPalette::default();

    // Setup: Set textarea text to "hello"
    state.textarea.insert_str("hello");

    // Action: Call update with Submit
    let cmds = crate::tui::update::update(&mut state, &mut palette, Msg::Submit);

    // Assert: Returns Cmd::SpawnAgent
    assert!(!cmds.is_empty(), "Should return commands");
    assert!(cmds.iter().any(|c| matches!(c, Cmd::SpawnAgent { .. })), "Should return SpawnAgent cmd");

    // Assert: agent_running is true
    assert!(state.agent_running, "agent_running should be true after submit");

    // Assert: Messages has User("hello") + Assistant(empty)
    assert_eq!(state.messages.len(), 2, "Should have 2 messages");
    assert!(matches!(&state.messages[0], MessageItem::User { text, .. } if text == "hello"));
    assert!(matches!(&state.messages[1], MessageItem::Assistant { text, .. } if text.is_empty()));

    // Assert: thinking is Some
    assert!(state.thinking.is_some(), "thinking should be Some after submit");

    // Simulate agent events
    handle_agent_event(&mut state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::MessageUpdate {
        message: agent_message("assistant", "Hi!"),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::MessageEnd {
        message: agent_message("assistant", "Hi!"),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: runie_agent::TokenUsage::default(),
    });

    // Assert final state
    assert!(!state.agent_running, "agent_running should be false after AgentEnd");
    assert!(state.thinking.is_none(), "thinking should be None after AgentEnd");

    // Assert assistant text is "Hi!"
    let assistant_text = state.messages.iter()
        .find_map(|m| if let MessageItem::Assistant { text, .. } = m { Some(text.clone()) } else { None });
    assert_eq!(assistant_text, Some("Hi!".to_string()), "Assistant text should be 'Hi!'");

    // Assert ViewModel has correct message list
    let vm = build_viewmodels(&state);
    assert!(vm.message_list.feed.items().iter().any(|item| {
        matches!(item, FeedItem::AssistantMessage { text, .. } if text.contains("Hi"))
    }), "ViewModel should have assistant message with 'Hi!'");
}

// ─── Test 2: Full Cycle With Tool Call ─────────────────────────────────────────

#[test]
fn test_full_cycle_with_tool_call() {
    let mut state = make_test_state();
    let mut palette = CommandPalette::default();

    // Setup: textarea text = "calculate 15+27"
    submit_message(&mut state, &mut palette, "calculate 15+27");

    // Simulate tool call flow
    handle_agent_event(&mut state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::MessageUpdate {
        message: AgentMessage {
            role: "assistant".to_string(),
            content: vec![ContentPart::Text { text: String::new() }],
            timestamp: 0,
            usage: None,
            stop_reason: None,
            error_message: None,
            tool_calls: vec![],
        },
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::ToolExecutionStart {
        tool_call_id: "call_calc_1".to_string(),
        tool_name: "calculator".to_string(),
        tool_args: "{\"expression\": \"15 + 27\"}".to_string(),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::ToolExecutionEnd {
        tool_call_id: "call_calc_1".to_string(),
        tool_name: "calculator".to_string(),
        tool_args: "{\"expression\": \"15 + 27\"}".to_string(),
        result: ToolResult {
            tool_call_id: "call_calc_1".to_string(),
            tool_name: "calculator".to_string(),
            input: serde_json::json!({"expression": "15 + 27"}),
            content: vec![ContentPart::Text { text: "42".to_string() }],
            is_error: false,
        },
        duration_ms: 50,
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::MessageUpdate {
        message: agent_message("assistant", "The result is 42."),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::MessageEnd {
        message: agent_message("assistant", "The result is 42."),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: runie_agent::TokenUsage::default(),
    });

    // Assert: Messages: User → Assistant → ToolCall
    assert!(matches!(&state.messages[0], MessageItem::User { .. }));
    assert!(matches!(&state.messages[1], MessageItem::Assistant { .. }));
    assert!(state.messages.iter().any(|m| matches!(m, MessageItem::ToolCall { .. })), "Should have ToolCall");

    // Assert: ToolCall has name and result
    let tool_call = state.messages.iter()
        .find_map(|m| if let MessageItem::ToolCall { name, result, .. } = m {
            Some((name.clone(), result.clone()))
        } else { None });
    assert!(tool_call.is_some(), "Should have ToolCall item");
    let (name, result) = tool_call.unwrap();
    assert!(name.contains("call_calc_1"), "ToolCall should have id");
    assert_eq!(result, Some("42".to_string()), "ToolCall should have result '42'");

    // Note: ToolCall items are filtered out in Feed conversion (they become Err(()))
    // So we check state.messages directly for tool presence
    assert!(state.messages.iter().any(|m| matches!(m, MessageItem::ToolCall { .. })),
        "state.messages should have ToolCall item");
}

// ─── Test 3: Full Cycle Streaming Response ─────────────────────────────────────

#[test]
fn test_full_cycle_streaming_response() {
    let mut state = make_test_state();
    let mut palette = CommandPalette::default();

    // Setup: textarea text = "count to 5"
    submit_message(&mut state, &mut palette, "count to 5");

    // Simulate streaming response
    handle_agent_event(&mut state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 0,
    });

    // During streaming: agent_running stays true
    assert!(state.agent_running, "agent_running should stay true during streaming");

    handle_agent_event(&mut state, AgentEvent::MessageUpdate {
        message: agent_message("assistant", "1"),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::MessageUpdate {
        message: agent_message("assistant", "1, 2"),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::MessageUpdate {
        message: agent_message("assistant", "1, 2, 3"),
        turn: 0,
    });

    // Assert: Assistant text is "1, 2, 3" (accumulated)
    let assistant_text = state.messages.iter()
        .find_map(|m| if let MessageItem::Assistant { text, .. } = m { Some(text.clone()) } else { None });
    assert_eq!(assistant_text, Some("1, 2, 3".to_string()), "Assistant text should be accumulated '1, 2, 3'");

    handle_agent_event(&mut state, AgentEvent::MessageEnd {
        message: agent_message("assistant", "1, 2, 3"),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: runie_agent::TokenUsage::default(),
    });

    // Assert: After AgentEnd, agent_running is false
    assert!(!state.agent_running, "agent_running should be false after AgentEnd");
}

// ─── Test 4: Full Cycle Error During Streaming ─────────────────────────────────

#[test]
fn test_full_cycle_error_during_streaming() {
    let mut state = make_test_state();
    let mut palette = CommandPalette::default();

    // Setup: textarea text = "error test"
    submit_message(&mut state, &mut palette, "error test");

    // Simulate partial streaming then error
    handle_agent_event(&mut state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::MessageUpdate {
        message: agent_message("assistant", "partial"),
        turn: 0,
    });

    // Error occurs
    handle_agent_event(&mut state, AgentEvent::Error {
        message: "Connection reset by peer".to_string(),
        error_type: "network_error".to_string(),
        recoverable: true,
        context: "during streaming".to_string(),
    });
    handle_agent_event(&mut state, AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: runie_agent::TokenUsage::default(),
    });

    // Assert: Error message in feed
    assert!(state.messages.iter().any(|m| matches!(m, MessageItem::Error { .. })),
        "Should have Error message in feed");

    // Assert: agent_running is false after error
    assert!(!state.agent_running, "agent_running should be false after error");

    // Assert: ViewModel shows error
    let vm = build_viewmodels(&state);
    assert!(vm.message_list.feed.items().iter().any(|item| {
        matches!(item, FeedItem::SystemNotice { text } if text.contains("Error") || text.contains("error"))
    }), "ViewModel should show error notice");
}

// ─── Test 5: Full Cycle Permission Gate ────────────────────────────────────────

#[test]
fn test_full_cycle_permission_gate() {
    let mut state = make_test_state();
    let mut palette = CommandPalette::default();

    // Setup: textarea text = "run bash command"
    submit_message(&mut state, &mut palette, "run bash command");

    // Simulate permission request flow
    handle_agent_event(&mut state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::MessageUpdate {
        message: agent_message("assistant", "Let me run that bash command for you."),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::ToolExecutionStart {
        tool_call_id: "call_bash_1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "{\"command\": \"ls -la\"}".to_string(),
        turn: 0,
    });

    // This triggers permission request
    handle_agent_event(&mut state, AgentEvent::PermissionRequest {
        tool_call_id: "call_bash_1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "{\"command\": \"ls -la\"}".to_string(),
        tool_description: "Execute a bash command".to_string(),
        turn: 0,
        context_window_usage: 0.3,
    });

    // Assert: After PermissionRequest, state.mode is Permission
    assert_eq!(state.mode, TuiMode::Permission, "mode should be Permission after permission request");

    // Simulate permission confirmation
    let msg = Msg::PermissionConfirm;
    let cmds = crate::tui::update::update(&mut state, &mut palette, msg);

    // Assert: After PermissionConfirm, mode returns to Chat
    assert_eq!(state.mode, TuiMode::Chat, "mode should return to Chat after confirm");

    // Assert: Commands include SendPermission
    assert!(cmds.iter().any(|c| matches!(c, Cmd::SendPermission { .. })), "Should send permission decision");

    // Simulate tool execution completion
    handle_agent_event(&mut state, AgentEvent::ToolExecutionEnd {
        tool_call_id: "call_bash_1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "{\"command\": \"ls -la\"}".to_string(),
        result: ToolResult {
            tool_call_id: "call_bash_1".to_string(),
            tool_name: "bash".to_string(),
            input: serde_json::json!({"command": "ls -la"}),
            content: vec![ContentPart::Text { text: "total 32\ndrwxr-xr-x ...".to_string() }],
            is_error: false,
        },
        duration_ms: 100,
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::MessageEnd {
        message: agent_message("assistant", "Here's the result of the bash command:"),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: runie_agent::TokenUsage::default(),
    });

    // Assert: Tool result is present
    assert!(state.messages.iter().any(|m| {
        if let MessageItem::ToolCall { result, .. } = m {
            result.is_some()
        } else { false }
    }), "Tool should have result after execution");
}

// ─── Test 6: Full Cycle Multiple Turns ─────────────────────────────────────────

#[test]
fn test_full_cycle_multiple_turns() {
    let mut state = make_test_state();
    let mut palette = CommandPalette::default();

    // ===== Turn 1 =====
    submit_message(&mut state, &mut palette, "question 1");

    handle_agent_event(&mut state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::MessageUpdate {
        message: agent_message("assistant", "answer 1"),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::MessageEnd {
        message: agent_message("assistant", "answer 1"),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::TurnEnd {
        turn: 0,
        message_count: 2,
        tool_results_count: 0,
        token_usage: runie_agent::TokenUsage::default(),
    });
    // Token usage comes via separate TokenUsage events, not from AgentEnd.final_token_usage
    handle_agent_event(&mut state, AgentEvent::TokenUsage {
        prompt_tokens: 10,
        completion_tokens: 20,
        total_tokens: 30,
        context_window: 128_000,
    });
    handle_agent_event(&mut state, AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: runie_agent::TokenUsage::default(),
    });

    // Verify turn 1: User + Assistant (no Separator in messages - turn info stored in state fields)
    assert_eq!(state.messages.len(), 2, "Should have 2 messages after turn 1 (User + Assistant)");
    assert!(matches!(&state.messages[0], MessageItem::User { text, .. } if text == "question 1"));
    assert!(matches!(&state.messages[1], MessageItem::Assistant { text, .. } if text == "answer 1"));
    // Turn info stored in state fields
    assert!(state.last_turn_duration_secs.is_some(), "last_turn_duration_secs should be set after turn 1");
    assert!(state.last_turn_tokens.is_some(), "last_turn_tokens should be set after turn 1");
    assert!(!state.agent_running, "agent_running should be false after turn 1");

    // ===== Turn 2 =====
    submit_message(&mut state, &mut palette, "question 2");

    handle_agent_event(&mut state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 1,
    });
    handle_agent_event(&mut state, AgentEvent::MessageUpdate {
        message: agent_message("assistant", "answer 2"),
        turn: 1,
    });
    handle_agent_event(&mut state, AgentEvent::MessageEnd {
        message: agent_message("assistant", "answer 2"),
        turn: 1,
    });
    handle_agent_event(&mut state, AgentEvent::TurnEnd {
        turn: 1,
        message_count: 2,
        tool_results_count: 0,
        token_usage: runie_agent::TokenUsage::default(),
    });
    // Token usage via separate events
    handle_agent_event(&mut state, AgentEvent::TokenUsage {
        prompt_tokens: 15,
        completion_tokens: 25,
        total_tokens: 40,
        context_window: 128_000,
    });
    handle_agent_event(&mut state, AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 2,
        final_token_usage: runie_agent::TokenUsage::default(),
    });

    // Assert: Messages: User1 → Assistant1 → User2 → Assistant2 (no Separators - turn info in state fields)
    assert!(state.messages.len() >= 4, "Should have at least 4 messages after turn 2");

    // Assert: Token usage accumulates via TokenUsage events
    assert_eq!(state.session_token_usage.total_tokens, 70, "Token usage should accumulate (30 + 40)");
}

// ─── Test 7: Full Cycle Thinking Shown Then Collapsed ───────────────────────────

#[test]
fn test_full_cycle_thinking_shown_then_collapsed() {
    let mut state = make_test_state();
    let mut palette = CommandPalette::default();

    // Setup: textarea text = "explain quantum physics"
    submit_message(&mut state, &mut palette, "explain quantum physics");

    // Simulate thinking then response
    handle_agent_event(&mut state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::ThinkingStart { turn: 0 });
    handle_agent_event(&mut state, AgentEvent::ThinkingUpdate {
        text: "This is a complex topic about physics...".to_string(),
        turn: 0,
    });
    // Sleep to ensure thinking duration is significant
    std::thread::sleep(std::time::Duration::from_millis(100));
    handle_agent_event(&mut state, AgentEvent::ThinkingEnd {
        duration_ms: 100,
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::MessageUpdate {
        message: agent_message("assistant", "Quantum physics is..."),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::MessageEnd {
        message: agent_message("assistant", "Quantum physics is the study of matter and energy at the most fundamental level."),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: runie_agent::TokenUsage::default(),
    });

    // Assert: Thought item is present in state.messages
    assert!(state.messages.iter().any(|m| matches!(m, MessageItem::Thought { .. })),
        "Should have Thought item in state.messages");

    // Assert: Assistant text doesn't contain thinking text
    let assistant_text = state.messages.iter()
        .find_map(|m| if let MessageItem::Assistant { text, .. } = m { Some(text.clone()) } else { None });
    assert!(assistant_text.is_some(), "Should have assistant text");
    assert!(!assistant_text.unwrap().contains("complex topic"), "Assistant text should not contain thinking");

    // Note: Thought items are filtered out in Feed conversion (they become Err(()))
    // So we check state.messages directly for thought presence
    // ViewModel's feed should still have the assistant message
    let vm = build_viewmodels(&state);
    assert!(vm.message_list.feed.items().iter().any(|item| {
        matches!(item, FeedItem::AssistantMessage { .. })
    }), "ViewModel should have assistant message");
}

// ─── Test 8: Full Cycle State Consistency ─────────────────────────────────────

#[test]
fn test_full_cycle_state_consistency() {
    // Run complete cycle 3 times
    for iteration in 0..3 {
        let mut state = make_test_state();
        let mut palette = CommandPalette::default();

        // Submit and complete a cycle
        let text = format!("message {}", iteration);
        submit_message(&mut state, &mut palette, &text);

        handle_agent_event(&mut state, AgentEvent::MessageStart {
            message: agent_message("assistant", ""),
            turn: 0,
        });
        handle_agent_event(&mut state, AgentEvent::MessageUpdate {
            message: agent_message("assistant", &format!("response {}", iteration)),
            turn: 0,
        });
        handle_agent_event(&mut state, AgentEvent::MessageEnd {
            message: agent_message("assistant", &format!("response {}", iteration)),
            turn: 0,
        });
        handle_agent_event(&mut state, AgentEvent::AgentEnd {
            messages: vec![],
            total_turns: 1,
            final_token_usage: runie_agent::TokenUsage::default(),
        });

        // Verify state consistency after each cycle
        assert!(!state.agent_running, "Iteration {}: agent_running should be false", iteration);
        assert!(state.thinking.is_none(), "Iteration {}: thinking should be None", iteration);
        assert!(state.status_header.is_none(), "Iteration {}: status_header should be None", iteration);
        assert!(state.status_start_time.is_none(), "Iteration {}: status_start_time should be None", iteration);

        // Check no duplicate empty assistant placeholders
        let empty_assistants = state.messages.iter()
            .filter(|m| matches!(m, MessageItem::Assistant { text, .. } if text.is_empty()))
            .count();
        assert_eq!(empty_assistants, 0, "Iteration {}: Should have no empty assistant placeholders", iteration);

        // Verify ViewModel builds without issues
        let vm = build_viewmodels(&state);
        assert!(vm.message_list.feed.items().iter().any(|item| {
            matches!(item, FeedItem::AssistantMessage { .. })
        }), "Iteration {}: ViewModel should have assistant message", iteration);
    }
}

// ─── Additional Integration Tests ───────────────────────────────────────────────

#[test]
fn test_full_cycle_agent_spawns_correct_messages() {
    let mut state = make_test_state();
    let mut palette = CommandPalette::default();

    // Pre-populate with a user message
    state.messages.push(MessageItem::User {
        text: "previous message".to_string(),
        model: Some("You".to_string()),
        timestamp: None,
    });

    // Submit new message
    let cmds = submit_message(&mut state, &mut palette, "new message");

    // Assert: SpawnAgent contains user messages (to_agent_messages includes empty assistant placeholder)
    if let Cmd::SpawnAgent { messages } = &cmds[0] {
        // state.messages = [User(prev), User(new), Assistant("")]
        // to_agent_messages converts all User and Assistant items = 3 messages
        assert_eq!(messages.len(), 3, "Should have 3 messages for agent");
        assert_eq!(messages[0].role, "user", "First should be user");
        assert!(messages[0].content.iter().any(|p| {
            if let ContentPart::Text { text } = p { text.contains("previous") } else { false }
        }), "First message should contain 'previous'");
        assert_eq!(messages[1].role, "user", "Second should be user (new message)");
        // Third is the empty assistant placeholder
        assert_eq!(messages[2].role, "assistant", "Third should be assistant placeholder");
    } else {
        panic!("Expected SpawnAgent command");
    }
}

#[test]
fn test_full_cycle_viewmodel_reflects_agent_running() {
    let mut state = make_test_state();
    let mut palette = CommandPalette::default();

    // Before submit
    let vm_idle = build_viewmodels(&state);
    assert!(!vm_idle.agent_list.agent_running, "Should show agent not running initially");

    // After submit, before agent end
    submit_message(&mut state, &mut palette, "test");
    let vm_running = build_viewmodels(&state);
    assert!(vm_running.agent_list.agent_running, "Should show agent running after submit");

    // After agent ends
    handle_agent_event(&mut state, AgentEvent::MessageStart {
        message: agent_message("assistant", ""),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::MessageEnd {
        message: agent_message("assistant", "done"),
        turn: 0,
    });
    handle_agent_event(&mut state, AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: runie_agent::TokenUsage::default(),
    });

    let vm_done = build_viewmodels(&state);
    assert!(!vm_done.agent_list.agent_running, "Should show agent not running after end");
}

#[test]
fn test_full_cycle_empty_submit_does_not_spawn() {
    let mut state = make_test_state();
    let mut palette = CommandPalette::default();

    // Submit empty message
    let cmds = crate::tui::update::update(&mut state, &mut palette, Msg::Submit);

    // Should not spawn agent
    assert!(cmds.is_empty() || !cmds.iter().any(|c| matches!(c, Cmd::SpawnAgent { .. })),
        "Should not spawn agent for empty submit");

    // Should show error hint
    assert!(!state.input_right_info.is_empty() || state.messages.iter().any(|m| matches!(m, MessageItem::System { .. })),
        "Should show hint for empty submit");
}
