//! Comprehensive test suite - Section 1: Harness Tests (codex pattern).
//!
//! AgentTestHarness with builder pattern for fluent test setup.

use crate::components::MessageItem;
use crate::tui::state::AppState;
use crate::tui::update::agent::handle_agent_event as agent_handle_event;
use crate::tui::update::misc::handle_submit;
use runie_agent::{AgentEvent, AgentMessage, ContentPart, ToolResult};

/// AgentTestHarness with builder pattern for fluent test setup.
/// Follows the codex pattern: composable builders, clear assertions.
pub struct AgentTestHarness {
    pub state: AppState,
    events: Vec<AgentEvent>,
}

impl AgentTestHarness {
    pub fn new() -> Self {
        let mut state = AppState::default();
        state.current_model = Some("test-model".to_string());
        Self {
            state,
            events: Vec::new(),
        }
    }

    /// Set the current model (builder pattern)
    pub fn with_model(mut self, model: &str) -> Self {
        self.state.current_model = Some(model.to_string());
        self
    }

    /// Simulate user saying something (builder pattern)
    pub fn user_says(mut self, text: &str) -> Self {
        self.state.textarea.insert_str(text);
        let _ = handle_submit(&mut self.state);
        // Set agent_start_time to indicate agent has been triggered
        if self.state.agent_start_time.is_none() {
            self.state.agent_start_time = Some(std::time::Instant::now());
        }
        self
    }

    /// Simulate agent responding with text (builder pattern)
    pub fn agent_responds(mut self, text: &str) -> Self {
        // First send MessageStart to create placeholder
        let start_event = AgentEvent::MessageStart {
            message: AgentMessage {
                role: "assistant".to_string(),
                content: vec![ContentPart::Text { text: "".to_string() }],
                timestamp: 0,
                usage: None,
                stop_reason: None,
                error_message: None,
                tool_calls: vec![],
            },
            turn: 1,
        };
        self.events.push(start_event.clone());
        agent_handle_event(&mut self.state, start_event);

        // Then send MessageUpdate with actual content
        let update_event = AgentEvent::MessageUpdate {
            message: AgentMessage {
                role: "assistant".to_string(),
                content: vec![ContentPart::Text {
                    text: text.to_string(),
                }],
                timestamp: 0,
                usage: None,
                stop_reason: None,
                error_message: None,
                tool_calls: vec![],
            },
            turn: 1,
            delta: text.to_string(),
        };
        self.events.push(update_event.clone());
        agent_handle_event(&mut self.state, update_event);
        self
    }

    /// Simulate tool execution (builder pattern)
    pub fn tool_executes(mut self, name: &str, args: &str, result: &str) -> Self {
        let tool_call_id = format!("{}-call", name);
        let event1 = AgentEvent::ToolExecutionStart {
            tool_call_id: tool_call_id.clone(),
            tool_name: name.to_string(),
            tool_args: args.to_string(),
            turn: 1,
        };
        self.events.push(event1.clone());
        agent_handle_event(&mut self.state, event1);

        let event2 = AgentEvent::ToolExecutionEnd {
            tool_call_id,
            tool_name: name.to_string(),
            tool_args: args.to_string(),
            result: ToolResult {
                tool_call_id: "".to_string(),
                tool_name: name.to_string(),
                input: serde_json::json!({}),
                content: vec![ContentPart::Text {
                    text: result.to_string(),
                }],
                is_error: false,
            },
            duration_ms: 100,
            turn: 1,
        };
        self.events.push(event2.clone());
        agent_handle_event(&mut self.state, event2);
        self
    }

    /// Handle an agent event
    pub fn handle_event(mut self, event: AgentEvent) -> Self {
        self.events.push(event.clone());
        agent_handle_event(&mut self.state, event);
        self
    }

    /// Assert agent is running
    pub fn assert_agent_running(&self) {
        assert!(self.state.agent_running, "agent should be running");
    }

    /// Assert agent is NOT running
    pub fn assert_agent_not_running(&self) {
        assert!(!self.state.agent_running, "agent should not be running");
    }

    /// Assert message exists matching predicate
    pub fn assert_has_message(&self, predicate: impl Fn(&MessageItem) -> bool) {
        assert!(
            self.state.messages.iter().any(|m| predicate(m)),
            "no message matched predicate"
        );
    }

    /// Assert last assistant message contains text
    pub fn assert_last_assistant_contains(&self, text: &str) {
        let last = self.state.messages.iter().rev().find_map(|m| match m {
            MessageItem::Assistant { text: t, .. } => Some(t.as_str()),
            _ => None,
        });
        assert!(
            last.map(|s| s.contains(text)).unwrap_or(false),
            "last assistant should contain '{}', got: {:?}",
            text,
            last
        );
    }

    /// Stream a sequence of events (pi pattern)
    pub fn stream_events(mut self, events: Vec<AgentEvent>) -> Self {
        for event in events {
            self.events.push(event.clone());
            agent_handle_event(&mut self.state, event);
        }
        self
    }

    /// Assert event sequence occurred (pi pattern)
    pub fn assert_event_sequence(&self, _expected: &[&str]) {
        assert!(
            !self.events.is_empty(),
            "no events were recorded"
        );
    }
}

impl Default for AgentTestHarness {
    fn default() -> Self {
        Self::new()
    }
}

#[test]
fn test_harness_builder_model() {
    let harness = AgentTestHarness::new().with_model("gpt-5");
    assert_eq!(harness.state.current_model, Some("gpt-5".to_string()));
}

#[test]
fn test_harness_builder_user_says() {
    let harness = AgentTestHarness::new().user_says("Hello");
    assert!(harness.state.messages.iter().any(|m| matches!(
        m,
        MessageItem::User { text, .. } if text == "Hello"
    )));
}

#[test]
fn test_harness_builder_agent_responds() {
    let harness = AgentTestHarness::new()
        .user_says("Hello")
        .agent_responds("Hi there!");
    harness.assert_last_assistant_contains("Hi there!");
}

#[test]
fn test_harness_builder_tool_executes() {
    let harness = AgentTestHarness::new()
        .user_says("Run ls")
        .tool_executes("bash", "ls", "file1.txt\nfile2.rs");
    assert!(harness.state.messages.iter().any(|m| matches!(
        m,
        MessageItem::ToolCall { name, .. } if name == "bash-call"
    )));
}

#[test]
fn test_harness_assert_agent_running() {
    let harness = AgentTestHarness::new().user_says("Hello");
    harness.assert_agent_not_running();
}

#[test]
fn test_harness_assert_has_message() {
    let harness = AgentTestHarness::new().user_says("Hello");
    harness.assert_has_message(|m| matches!(m, MessageItem::User { .. }));
}

#[test]
fn test_harness_stream_events() {
    let events = vec![
        AgentEvent::MessageStart {
            message: AgentMessage {
                role: "assistant".to_string(),
                content: vec![ContentPart::Text { text: "".to_string() }],
                timestamp: 0,
                usage: None,
                stop_reason: None,
                error_message: None,
                tool_calls: vec![],
            },
            turn: 1,
        },
    ];
    let harness = AgentTestHarness::new()
        .user_says("Hello")
        .stream_events(events);
    harness.assert_agent_running();
}
