//! Comprehensive test suite - Section 3: Mock Stream Tests (pi pattern).

use crate::components::MessageItem;
use runie_agent::{AgentEvent, ContentPart, ToolResult, TokenUsage};

use super::harness::AgentTestHarness;
use super::state_tests::{make_message, token_usage};

#[test]
fn test_stream_event_sequence() {
    let events = vec![
        AgentEvent::MessageStart {
            message: make_message("assistant", ""),
            turn: 1,
        },
        AgentEvent::MessageUpdate {
            message: make_message("assistant", "Hello"),
            turn: 1,
            delta: "Hello".to_string(),
        },
        AgentEvent::ToolExecutionStart {
            tool_call_id: "t1".to_string(),
            tool_name: "bash".to_string(),
            tool_args: "ls".to_string(),
            turn: 1,
        },
        AgentEvent::ToolExecutionEnd {
            tool_call_id: "t1".to_string(),
            tool_name: "bash".to_string(),
            tool_args: "ls".to_string(),
            result: ToolResult {
                tool_call_id: "t1".to_string(),
                tool_name: "bash".to_string(),
                input: serde_json::json!({}),
                content: vec![ContentPart::Text {
                    text: "file1.txt".to_string(),
                }],
                is_error: false,
            },
            duration_ms: 100,
            turn: 1,
        },
        AgentEvent::MessageEnd {
            message: make_message("assistant", "Hello"),
            turn: 1,
        },
        AgentEvent::TurnEnd {
            turn: 1,
            message_count: 3,
            tool_results_count: 1,
            token_usage: TokenUsage {
                input: 100,
                output: 50,
                cache_read: 0,
                cache_write: 0,
                total_tokens: 150,
            },
        },
    ];

    let harness = AgentTestHarness::new()
        .user_says("Hello")
        .stream_events(events);

    harness.assert_event_sequence(&[
        "message_start",
        "message_update",
        "tool_start",
        "tool_end",
        "message_end",
        "turn_end",
    ]);
}

#[test]
fn test_stream_text_updates() {
    let harness = AgentTestHarness::new()
        .user_says("Hello")
        .agent_responds("Hi")
        .agent_responds("Hi there")
        .agent_responds("Hi there!");

    harness.assert_last_assistant_contains("Hi there!");
}

#[test]
fn test_stream_preserves_message_order() {
    let mut harness = AgentTestHarness::new();
    harness = harness.user_says("First");

    harness = harness.handle_event(AgentEvent::MessageStart {
        message: make_message("assistant", ""),
        turn: 1,
    });
    harness = harness.handle_event(AgentEvent::MessageUpdate {
        message: make_message("assistant", "Response to first"),
        turn: 1,
        delta: "Response to first".to_string(),
    });
    harness = harness.handle_event(AgentEvent::MessageEnd {
        message: make_message("assistant", "Response to first"),
        turn: 1,
    });
    harness = harness.handle_event(AgentEvent::TurnEnd {
        turn: 1,
        message_count: 2,
        tool_results_count: 0,
        token_usage: TokenUsage {
            input: 50,
            output: 25,
            cache_read: 0,
            cache_write: 0,
            total_tokens: 75,
        },
    });

    harness = harness.user_says("Second");

    harness = harness.handle_event(AgentEvent::MessageStart {
        message: make_message("assistant", ""),
        turn: 2,
    });
    harness = harness.handle_event(AgentEvent::MessageUpdate {
        message: make_message("assistant", "Response to second"),
        turn: 2,
        delta: "Response to second".to_string(),
    });

    let user_messages: Vec<_> = harness
        .state
        .messages
        .iter()
        .filter(|m| matches!(m, MessageItem::User { .. }))
        .collect();

    assert_eq!(user_messages.len(), 2);
}

#[test]
fn test_stream_turn_separators() {
    let harness = AgentTestHarness::new()
        .user_says("Turn 1")
        .handle_event(AgentEvent::MessageStart {
            message: make_message("assistant", ""),
            turn: 1,
        })
        .handle_event(AgentEvent::MessageUpdate {
            message: make_message("assistant", "Response 1"),
            turn: 1,
            delta: "Response 1".to_string(),
        })
        .handle_event(AgentEvent::MessageEnd {
            message: make_message("assistant", "Response 1"),
            turn: 1,
        })
        .handle_event(AgentEvent::TurnEnd {
            turn: 1,
            message_count: 2,
            tool_results_count: 0,
            token_usage: TokenUsage {
                input: 50,
                output: 25,
                cache_read: 0,
                cache_write: 0,
                total_tokens: 75,
            },
        })
        .user_says("Turn 2")
        .handle_event(AgentEvent::MessageStart {
            message: make_message("assistant", ""),
            turn: 2,
        })
        .handle_event(AgentEvent::MessageUpdate {
            message: make_message("assistant", "Response 2"),
            turn: 2,
            delta: "Response 2".to_string(),
        })
        .handle_event(AgentEvent::MessageEnd {
            message: make_message("assistant", "Response 2"),
            turn: 2,
        })
        .handle_event(AgentEvent::TurnEnd {
            turn: 2,
            message_count: 2,
            tool_results_count: 0,
            token_usage: TokenUsage {
                input: 50,
                output: 25,
                cache_read: 0,
                cache_write: 0,
                total_tokens: 75,
            },
        });

    let separators: Vec<_> = harness
        .state
        .messages
        .iter()
        .filter(|m| matches!(m, MessageItem::Separator { .. }))
        .collect();

    assert_eq!(separators.len(), 2);
}

#[test]
fn test_stream_token_usage_accumulates() {
    let mut harness = AgentTestHarness::new();

    harness = harness.handle_event(token_usage(100, 50));
    harness = harness.handle_event(token_usage(200, 100));

    assert_eq!(harness.state.session_token_usage.prompt_tokens, 300);
    assert_eq!(harness.state.session_token_usage.completion_tokens, 150);
    assert_eq!(harness.state.session_token_usage.total_tokens, 450);
}

#[test]
fn test_token_usage_zero() {
    let harness = AgentTestHarness::new()
        .user_says("Hello")
        .handle_event(token_usage(0, 0));

    assert_eq!(harness.state.session_token_usage.total_tokens, 0);
}

#[test]
fn test_token_usage_large_numbers() {
    let harness = AgentTestHarness::new()
        .user_says("Hello")
        .handle_event(token_usage(100_000, 50_000));

    assert_eq!(harness.state.session_token_usage.total_tokens, 150_000);
}

#[test]
fn test_turn_count() {
    let harness = AgentTestHarness::new()
        .user_says("Turn 1")
        .handle_event(AgentEvent::MessageStart {
            message: make_message("assistant", ""),
            turn: 1,
        })
        .handle_event(AgentEvent::MessageUpdate {
            message: make_message("assistant", "Response X"),
            turn: 1,
            delta: "Response X".to_string(),
        })
        .handle_event(AgentEvent::MessageEnd {
            message: make_message("assistant", "Response X"),
            turn: 1,
        })
        .handle_event(AgentEvent::TurnEnd {
            turn: 1,
            message_count: 2,
            tool_results_count: 0,
            token_usage: TokenUsage {
                input: 50,
                output: 25,
                cache_read: 0,
                cache_write: 0,
                total_tokens: 75,
            },
        })
        .user_says("Turn 2")
        .handle_event(AgentEvent::MessageStart {
            message: make_message("assistant", ""),
            turn: 2,
        })
        .handle_event(AgentEvent::MessageUpdate {
            message: make_message("assistant", "Response X"),
            turn: 2,
            delta: "Response X".to_string(),
        })
        .handle_event(AgentEvent::MessageEnd {
            message: make_message("assistant", "Response X"),
            turn: 2,
        })
        .handle_event(AgentEvent::TurnEnd {
            turn: 2,
            message_count: 2,
            tool_results_count: 0,
            token_usage: TokenUsage {
                input: 50,
                output: 25,
                cache_read: 0,
                cache_write: 0,
                total_tokens: 75,
            },
        });

    let separators: Vec<_> = harness
        .state
        .messages
        .iter()
        .filter(|m| matches!(m, MessageItem::Separator { elapsed_secs: _, .. }))
        .collect();

    assert_eq!(separators.len(), 2);
}

#[test]
fn test_thinking_duration_accumulated() {
    let mut harness = AgentTestHarness::new();
    harness = harness.user_says("Hello");

    harness = harness.handle_event(AgentEvent::MessageStart {
        message: make_message("assistant", ""),
        turn: 1,
    });

    // Simulate some thinking time
    harness.state.thinking_start =
        Some(std::time::Instant::now() - std::time::Duration::from_millis(800));

    harness = harness.handle_event(AgentEvent::ToolExecutionStart {
        tool_call_id: "t1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "ls".to_string(),
        turn: 1,
    });

    // Thinking duration should be accumulated
    assert!(harness.state.thinking_duration.is_some());
    assert!(harness.state.thinking_duration.unwrap().as_millis() >= 700);
}

#[test]
fn test_message_content_extraction() {
    let harness = AgentTestHarness::new()
        .user_says("Hello")
        .handle_event(AgentEvent::Message {
            role: "assistant".to_string(),
            content: "Part1 Part2".to_string(),
        });

    let assistant_text = harness.state.messages.iter().find_map(|m| match m {
        MessageItem::Assistant { text, .. } => Some(text.as_str()),
        _ => None,
    });

    assert_eq!(assistant_text, Some("Part1 Part2"));
}
