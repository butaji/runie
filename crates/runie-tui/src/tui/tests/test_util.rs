//! Test utilities for TUI component testing.
//!
//! Provides fixtures, assertions, and helpers for testing TUI state management,
//! event processing, and UI components.

use std::fs;
use tempfile::TempDir;

use crate::components::MessageItem;
use crate::tui::state::AppState;
use runie_agent::{AgentEvent, AgentMessage, ContentPart, ToolResult, TokenUsage};

// ═══════════════════════════════════════════════════════════════════════════════
// TEMP DIRECTORY FIXTURE (opencode pattern)
// ═══════════════════════════════════════════════════════════════════════════════

/// RAII-style temporary directory that auto-cleans on drop.
pub struct TestTempDir {
    dir: TempDir,
}

impl TestTempDir {
    /// Create a new temporary directory.
    pub fn new() -> Self {
        Self {
            dir: TempDir::new().unwrap(),
        }
    }

    /// Get the path to the temporary directory.
    pub fn path(&self) -> &std::path::Path {
        self.dir.path()
    }

    /// Write a file to the temp directory and return its path.
    pub fn write_file(&self, name: &str, content: &str) -> std::path::PathBuf {
        let path = self.dir.path().join(name);
        fs::write(&path, content).unwrap();
        path
    }
}

impl Default for TestTempDir {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TestTempDir {
    fn drop(&mut self) {
        // Auto-cleaned by TempDir
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// EVENT SEQUENCE ASSERTIONS (codex pattern)
// ═══════════════════════════════════════════════════════════════════════════════

/// Assert that the sequence of agent events matches the expected event type names.
pub fn assert_event_sequence(events: &[AgentEvent], expected: &[&str]) {
    let actual: Vec<String> = events.iter().map(|e| event_type_name(e).to_string()).collect();
    assert_eq!(
        actual,
        expected.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        "Event sequence mismatch"
    );
}

/// Get the type name of an agent event as a static string.
pub fn event_type_name(event: &AgentEvent) -> &'static str {
    if matches!(event, AgentEvent::MessageStart { .. }) { return "message_start"; }
    if matches!(event, AgentEvent::MessageUpdate { .. }) { return "message_update"; }
    if matches!(event, AgentEvent::MessageEnd { .. }) { return "message_end"; }
    if matches!(event, AgentEvent::ToolExecutionStart { .. }) { return "tool_start"; }
    if matches!(event, AgentEvent::ToolExecutionEnd { .. }) { return "tool_end"; }
    if matches!(event, AgentEvent::TurnEnd { .. }) { return "turn_end"; }
    if matches!(event, AgentEvent::AgentEnd { .. }) { return "agent_end"; }
    if matches!(event, AgentEvent::Error { .. }) { return "error"; }
    "other"
}

// ═══════════════════════════════════════════════════════════════════════════════
// STATE ASSERTIONS (crush pattern)
// ═══════════════════════════════════════════════════════════════════════════════

/// Extension trait for asserting on AppState.
pub trait StateAssertions {
    fn assert_agent_running(&self);
    fn assert_agent_idle(&self);
    fn assert_has_assistant(&self, text: &str);
    fn assert_has_tool_call(&self, name: &str);
    fn assert_has_error(&self);
    fn assert_token_count(&self, expected: usize);
}

impl StateAssertions for AppState {
    fn assert_agent_running(&self) {
        assert!(self.agent_running, "agent should be running");
    }

    fn assert_agent_idle(&self) {
        assert!(!self.agent_running, "agent should be idle");
        assert!(self.status_header.is_none(), "status should be cleared");
    }

    fn assert_has_assistant(&self, expected_text: &str) {
        assert!(
            self.messages.iter().any(|m| matches!(
                m,
                MessageItem::Assistant { text, .. } if text.contains(expected_text)
            )),
            "should have assistant text containing: {}",
            expected_text
        );
    }

    fn assert_has_tool_call(&self, expected_name: &str) {
        assert!(
            self.messages.iter().any(|m| matches!(
                m,
                MessageItem::ToolCall { name, .. } if name == expected_name
            )),
            "should have tool call: {}",
            expected_name
        );
    }

    fn assert_has_error(&self) {
        assert!(
            self.messages.iter().any(|m| matches!(m, MessageItem::Error { .. })),
            "should have error message"
        );
    }

    fn assert_token_count(&self, expected: usize) {
        assert_eq!(
            self.session_token_usage.total_tokens as usize,
            expected,
            "token count mismatch"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// ASYNC TEST HELPERS (pi pattern)
// ═══════════════════════════════════════════════════════════════════════════════

use tokio::time::{sleep, Duration};

/// Wait for a predicate to become true, with timeout.
pub async fn wait_for_condition<F, Fut>(mut predicate: F, timeout_ms: u64)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let start = std::time::Instant::now();
    let timeout = Duration::from_millis(timeout_ms);

    while start.elapsed() < timeout {
        if predicate().await {
            return;
        }
        sleep(Duration::from_millis(10)).await;
    }

    panic!("Condition not met within {}ms", timeout_ms);
}

/// Wait for the agent to become idle.
pub async fn wait_for_agent_idle(state: &AppState, timeout_ms: u64) {
    wait_for_condition(|| async { !state.agent_running }, timeout_ms).await;
}

// ═══════════════════════════════════════════════════════════════════════════════
// MOCK EVENT BUILDERS
// ═══════════════════════════════════════════════════════════════════════════════

/// Create a MessageStart event.
pub fn message_start(text: &str) -> AgentEvent {
    AgentEvent::MessageStart {
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
    }
}

/// Create a MessageUpdate event.
pub fn message_update(delta: &str) -> AgentEvent {
    AgentEvent::MessageUpdate {
        message: AgentMessage {
            role: "assistant".to_string(),
            content: vec![ContentPart::Text {
                text: delta.to_string(),
            }],
            timestamp: 0,
            usage: None,
            stop_reason: None,
            error_message: None,
            tool_calls: vec![],
        },
        turn: 1,
        delta: delta.to_string(),
    }
}

/// Create a ToolExecutionStart event.
pub fn tool_start(name: &str, args: &str) -> AgentEvent {
    AgentEvent::ToolExecutionStart {
        tool_call_id: format!("tool_{}", name),
        tool_name: name.to_string(),
        tool_args: args.to_string(),
        turn: 1,
    }
}

/// Create a ToolExecutionEnd event.
pub fn tool_end(name: &str, result: &str) -> AgentEvent {
    AgentEvent::ToolExecutionEnd {
        tool_call_id: format!("tool_{}", name),
        tool_name: name.to_string(),
        tool_args: String::new(),
        result: ToolResult {
            tool_call_id: format!("tool_{}", name),
            tool_name: name.to_string(),
            input: serde_json::Value::Null,
            content: vec![ContentPart::Text {
                text: result.to_string(),
            }],
            is_error: false,
        },
        duration_ms: 100,
        turn: 1,
    }
}

/// Create a TurnEnd event.
pub fn turn_end(tokens: usize) -> AgentEvent {
    AgentEvent::TurnEnd {
        turn: 1,
        message_count: 2,
        tool_results_count: 0,
        token_usage: TokenUsage {
            input: 0,
            output: 0,
            cache_read: 0,
            cache_write: 0,
            total_tokens: tokens as u32,
        },
    }
}

/// Create an Error event.
pub fn agent_error(message: &str) -> AgentEvent {
    AgentEvent::Error {
        message: message.to_string(),
        error_type: "test".to_string(),
        recoverable: true,
        context: String::new(),
    }
}
