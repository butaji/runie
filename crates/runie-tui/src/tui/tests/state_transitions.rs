//! Table-driven tests for agent state transitions.
//!
//! Tests state changes in response to AgentEvent variants:
//! - Message lifecycle (start, update, end)
//! - Tool execution lifecycle (start, end)
//! - Agent lifecycle (agent end)
//! - Error handling
//! - Token usage accumulation

use crate::components::MessageItem;
use crate::tui::state::AppState;
use crate::tui::update::agent;
use runie_agent::{AgentEvent, AgentMessage, ContentPart::Text, TokenUsage as AgentTokenUsage};
use std::time::Instant;

/// Helper: Create an AgentMessage with given role and content text.
fn agent_message(role: &str, content: &str) -> AgentMessage {
    AgentMessage {
        role: role.to_string(),
        content: vec![Text { text: content.to_string() }],
        timestamp: 0,
        usage: None,
        stop_reason: None,
        error_message: None,
        tool_calls: vec![],
    }
}

/// Helper: Create a TokenUsage event with given prompt/completion.
fn token_usage(prompt: usize, completion: usize) -> AgentEvent {
    AgentEvent::TokenUsage {
        prompt_tokens: prompt,
        completion_tokens: completion,
        total_tokens: prompt + completion,
        context_window: 128_000,
    }
}

/// Table-driven test for state transitions.
#[test]
fn test_state_transitions() {
    struct TestCase {
        name: &'static str,
        initial: Box<dyn Fn(&mut AppState)>,
        event: AgentEvent,
        assertions: Box<dyn Fn(&AppState)>,
    }

    let cases = vec![
        TestCase {
            name: "message start sets thinking",
            initial: Box::new(|state| {
                state.agent_running = true;
            }),
            event: AgentEvent::MessageStart {
                message: agent_message("assistant", ""),
                turn: 1,
            },
            assertions: Box::new(|state| {
                assert!(state.is_thinking, "is_thinking should be true");
                assert_eq!(state.status_header, Some("Thinking".to_string()), "status_header");
            }),
        },
        TestCase {
            name: "tool start pauses thinking",
            initial: Box::new(|state| {
                state.agent_running = true;
                state.is_thinking = true;
                state.thinking_start = Some(Instant::now());
            }),
            event: AgentEvent::ToolExecutionStart {
                tool_call_id: "t1".to_string(),
                tool_name: "bash".to_string(),
                tool_args: "ls".to_string(),
                turn: 1,
            },
            assertions: Box::new(|state| {
                assert!(!state.is_thinking, "is_thinking should be false");
                assert!(state.thinking_duration.is_some(), "thinking_duration should be set");
                assert_eq!(state.status_header, Some("Working".to_string()), "status_header");
            }),
        },
        TestCase {
            name: "agent end clears all",
            initial: Box::new(|state| {
                state.agent_running = true;
                state.is_thinking = true;
                state.thinking_start = Some(Instant::now());
                state.status_header = Some("Thinking".to_string());
                state.status_start_time = Some(Instant::now());
            }),
            event: AgentEvent::AgentEnd {
                messages: vec![],
                total_turns: 1,
                final_token_usage: AgentTokenUsage::default(),
            },
            assertions: Box::new(|state| {
                assert!(!state.agent_running, "agent_running should be false");
                assert!(!state.is_thinking, "is_thinking should be false");
                assert!(state.thinking_start.is_none(), "thinking_start should be none");
                assert!(state.status_header.is_none(), "status_header should be none");
                assert!(state.status_start_time.is_none(), "status_start_time should be none");
            }),
        },
        TestCase {
            name: "error clears running but leaves message",
            initial: Box::new(|state| {
                state.agent_running = true;
                state.messages.push(MessageItem::Assistant {
                    text: "".to_string(),
                    model: None,
                    timestamp: None,
                });
            }),
            event: AgentEvent::Error {
                message: "fail".to_string(),
                error_type: "test".to_string(),
                recoverable: true,
                context: "".to_string(),
            },
            assertions: Box::new(|state| {
                assert!(!state.agent_running, "agent_running should be false");
                assert!(
                    state.messages.iter().any(|m| matches!(m, MessageItem::Error { .. })),
                    "should have error message"
                );
            }),
        },
    ];

    for case in cases {
        let mut state = AppState::default();
        (case.initial)(&mut state);
        agent::handle_agent_event(&mut state, case.event);
        (case.assertions)(&state);
    }
}

/// Test: Token usage accumulation.
#[test]
fn test_token_usage_accumulation() {
    struct TestCase {
        name: &'static str,
        events: Vec<AgentEvent>,
        expected_prompt: usize,
        expected_completion: usize,
        expected_total: usize,
    }

    let cases = vec![
        TestCase {
            name: "single usage event",
            events: vec![token_usage(10, 20)],
            expected_prompt: 10,
            expected_completion: 20,
            expected_total: 30,
        },
        TestCase {
            name: "multiple usage events accumulate",
            events: vec![token_usage(10, 20), token_usage(5, 10)],
            expected_prompt: 15,
            expected_completion: 30,
            expected_total: 45,
        },
        TestCase {
            name: "zero tokens",
            events: vec![token_usage(0, 0)],
            expected_prompt: 0,
            expected_completion: 0,
            expected_total: 0,
        },
    ];

    for case in cases {
        let mut state = AppState::default();
        for event in case.events {
            agent::handle_agent_event(&mut state, event);
        }
        assert_eq!(
            state.session_token_usage.prompt_tokens, case.expected_prompt,
            "{}: prompt", case.name
        );
        assert_eq!(
            state.session_token_usage.completion_tokens, case.expected_completion,
            "{}: completion", case.name
        );
        assert_eq!(
            state.session_token_usage.total_tokens, case.expected_total,
            "{}: total", case.name
        );
    }
}

/// Test: Message lifecycle transitions.
#[test]
fn test_message_lifecycle() {
    struct TestCase {
        name: &'static str,
        initial: Box<dyn Fn(&mut AppState)>,
        events: Vec<AgentEvent>,
        assertions: Box<dyn Fn(&AppState, &[String])>,
    }

    let assistant_texts = vec![
        "Hello".to_string(),
        "World".to_string(),
    ];

    let cases = vec![
        TestCase {
            name: "message start adds placeholder",
            initial: Box::new(|state| {
                state.agent_running = true;
            }),
            events: vec![AgentEvent::MessageStart {
                message: agent_message("assistant", ""),
                turn: 1,
            }],
            assertions: Box::new(|state, _texts| {
                assert!(
                    state.messages.iter().any(|m| matches!(
                        m,
                        MessageItem::Assistant {
                            text,
                            ..
                        } if text.is_empty()
                    )),
                    "should have empty assistant placeholder"
                );
            }),
        },
        TestCase {
            name: "message update fills placeholder",
            initial: Box::new(|_state| {}),
            events: vec![
                AgentEvent::MessageStart {
                    message: agent_message("assistant", ""),
                    turn: 1,
                },
                AgentEvent::MessageUpdate {
                    message: agent_message("assistant", "Hello"),
                    turn: 1,
                    delta: "Hello".to_string(),
                },
            ],
            assertions: Box::new(|state, _texts| {
                let has_hello = state.messages.iter().any(|m| matches!(
                    m,
                    MessageItem::Assistant {
                        text,
                        ..
                    } if text.contains("Hello")
                ));
                assert!(has_hello, "should have Hello in messages");
            }),
        },
    ];

    for case in cases {
        let mut state = AppState::default();
        (case.initial)(&mut state);
        for event in case.events {
            agent::handle_agent_event(&mut state, event);
        }
        (case.assertions)(&state, &assistant_texts);
    }
}

/// Test: Tool execution transitions.
#[test]
fn test_tool_execution_transitions() {
    struct TestCase {
        name: &'static str,
        initial: Box<dyn Fn(&mut AppState)>,
        event: AgentEvent,
        expect_tool_call: bool,
        expect_working: bool,
    }

    let cases = vec![
        TestCase {
            name: "tool start adds tool call message",
            initial: Box::new(|state| {
                state.agent_running = true;
                state.is_thinking = true;
            }),
            event: AgentEvent::ToolExecutionStart {
                tool_call_id: "tool_1".to_string(),
                tool_name: "bash".to_string(),
                tool_args: "ls -la".to_string(),
                turn: 1,
            },
            expect_tool_call: true,
            expect_working: true,
        },
        TestCase {
            name: "tool end updates tool call result",
            initial: Box::new(|state| {
                state.agent_running = true;
                state.messages.push(MessageItem::ToolCall {
                    name: "tool_1".to_string(),
                    args: "ls".to_string(),
                    result: None,
                    is_error: false,
                });
            }),
            event: AgentEvent::ToolExecutionEnd {
                tool_call_id: "tool_1".to_string(),
                tool_name: "bash".to_string(),
                tool_args: "ls".to_string(),
                result: runie_agent::events::ToolResult {
                    tool_call_id: "tool_1".to_string(),
                    tool_name: "bash".to_string(),
                    input: serde_json::json!({}),
                    content: vec![Text { text: "file1.txt\nfile2.rs".to_string() }],
                    is_error: false,
                },
                duration_ms: 100,
                turn: 1,
            },
            expect_tool_call: true,
            expect_working: false,
        },
    ];

    for case in cases {
        let mut state = AppState::default();
        (case.initial)(&mut state);
        agent::handle_agent_event(&mut state, case.event);

        let has_tool_call = state
            .messages
            .iter()
            .any(|m| matches!(m, MessageItem::ToolCall { .. }));
        assert_eq!(
            has_tool_call, case.expect_tool_call,
            "{}: tool_call present", case.name
        );

        assert_eq!(
            state.status_header.as_ref().map(|s| s.as_str()),
            if case.expect_working { Some("Working") } else { None },
            "{}: status_header", case.name
        );
    }
}
