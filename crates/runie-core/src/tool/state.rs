//! Tool call state machine for UI display.

use std::time::Instant;
use serde_json::Value;

#[cfg(test)]
use serde_json::json;

/// Tracks the lifecycle of a single tool call for UI display.
#[derive(Debug, Clone)]
pub enum ToolCallState {
    /// Tool call has been invoked, waiting for first response.
    Pending { id: String, name: String, input: Value },
    /// Tool is currently executing.
    Running { id: String, name: String, input: Value, started: Instant },
    /// Tool completed successfully.
    Completed { id: String, name: String, output: String, bytes: Option<u64>, duration_secs: f64 },
    /// Tool encountered an error.
    Error { id: String, name: String, error: String, duration_secs: f64 },
}

impl ToolCallState {
    /// Start the tool (transition from Pending to Running).
    pub fn start(&mut self) {
        if let ToolCallState::Pending { id, name, input } = self.clone() {
            *self = ToolCallState::Running { id, name, input, started: Instant::now() };
        }
    }

    /// Complete the tool (transition from Running to Completed).
    pub fn complete(&mut self, output: String, bytes: Option<u64>) {
        if let ToolCallState::Running { id, name, started, .. } = self.clone() {
            let duration_secs = started.elapsed().as_secs_f64();
            *self = ToolCallState::Completed { id, name, output, bytes, duration_secs };
        }
    }

    /// Mark tool as errored (transition from Running to Error).
    pub fn fail(&mut self, error: String) {
        if let ToolCallState::Running { id, name, started, .. } = self.clone() {
            let duration_secs = started.elapsed().as_secs_f64();
            *self = ToolCallState::Error { id, name, error, duration_secs };
        }
    }

    /// Get the tool call ID.
    pub fn id(&self) -> &str {
        match self {
            ToolCallState::Pending { id, .. } => id,
            ToolCallState::Running { id, .. } => id,
            ToolCallState::Completed { id, .. } => id,
            ToolCallState::Error { id, .. } => id,
        }
    }

    /// Get the tool name.
    pub fn name(&self) -> &str {
        match self {
            ToolCallState::Pending { name, .. } => name,
            ToolCallState::Running { name, .. } => name,
            ToolCallState::Completed { name, .. } => name,
            ToolCallState::Error { name, .. } => name,
        }
    }

    /// Check if this tool call matches another (for coalescing).
    pub fn matches(&self, name: &str, input: &Value) -> bool {
        match self {
            ToolCallState::Pending { name: n, input: i, .. } => n == name && i == input,
            ToolCallState::Running { name: n, input: i, .. } => n == name && i == input,
            _ => false,
        }
    }
}

/// Manages a collection of tool call states with coalescing support.
#[derive(Debug, Default)]
pub struct ToolCallTracker {
    calls: std::collections::HashMap<String, ToolCallState>,
}

impl ToolCallTracker {
    pub fn new() -> Self { Self::default() }

    /// Add a new pending tool call.
    pub fn add(&mut self, id: String, name: String, input: Value) {
        self.calls.insert(id.clone(), ToolCallState::Pending { id, name, input });
    }

    /// Start a pending tool call.
    pub fn start(&mut self, id: &str) {
        if let Some(state) = self.calls.get_mut(id) {
            state.start();
        }
    }

    /// Complete a running tool call.
    pub fn complete(&mut self, id: &str, output: String, bytes: Option<u64>) {
        if let Some(state) = self.calls.get_mut(id) {
            state.complete(output, bytes);
        }
    }

    /// Mark a running tool call as failed.
    pub fn fail(&mut self, id: &str, error: String) {
        if let Some(state) = self.calls.get_mut(id) {
            state.fail(error);
        }
    }

    /// Get a tool call state by ID.
    pub fn get(&self, id: &str) -> Option<&ToolCallState> {
        self.calls.get(id)
    }

    /// Get all tool call states.
    pub fn all(&self) -> impl Iterator<Item = &ToolCallState> {
        self.calls.values()
    }

    /// Count how many times an identical call (name + input) appears consecutively.
    pub fn coalesce_count(&self, name: &str, input: &Value) -> usize {
        self.calls.values().filter(|s| s.matches(name, input)).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_state_transitions_pending_to_running() {
        let mut state = ToolCallState::Pending {
            id: "call.1".into(),
            name: "bash".into(),
            input: json!({"command": "ls"}),
        };
        state.start();
        match state {
            ToolCallState::Running { id, name, started, .. } => {
                assert_eq!(id, "call.1");
                assert_eq!(name, "bash");
                assert!(started.elapsed().as_secs() >= 0);
            }
            _ => panic!("Expected Running state"),
        }
    }

    #[test]
    fn tool_state_transitions_running_to_completed() {
        let mut state = ToolCallState::Running {
            id: "call.1".into(),
            name: "bash".into(),
            input: json!({"command": "ls"}),
            started: Instant::now(),
        };
        state.complete("files listed".into(), None);
        match state {
            ToolCallState::Completed { id, output, duration_secs, .. } => {
                assert_eq!(id, "call.1");
                assert_eq!(output, "files listed");
                assert!(duration_secs >= 0.0);
            }
            _ => panic!("Expected Completed state"),
        }
    }

    #[test]
    fn tool_state_records_duration() {
        // Create a Running state with started time 100ms in the past
        let started = Instant::now() - std::time::Duration::from_millis(100);
        let mut state = ToolCallState::Running {
            id: "call.1".into(),
            name: "sleep".into(),
            input: json!({"seconds": 1}),
            started,
        };
        state.complete("done".into(), None);
        if let ToolCallState::Completed { duration_secs, .. } = state {
            // Duration should be approximately 0.1s (within 50ms tolerance)
            assert!(
                (duration_secs - 0.1).abs() < 0.05,
                "Duration should be ~0.1s, got {}",
                duration_secs
            );
        } else {
            panic!("Expected Completed");
        }
    }

    #[test]
    fn identical_calls_match() {
        let input = json!({"path": "src/lib.rs"});
        let state = ToolCallState::Pending {
            id: "c1".into(),
            name: "read_file".into(),
            input: input.clone(),
        };
        assert!(state.matches("read_file", &input));
        assert!(!state.matches("bash", &input));
    }

    #[test]
    fn tracker_add_and_get() {
        let mut tracker = ToolCallTracker::new();
        tracker.add("call.1".into(), "bash".into(), json!({"cmd": "ls"}));
        let state = tracker.get("call.1").unwrap();
        assert_eq!(state.name(), "bash");
    }

    #[test]
    fn tracker_complete_flow() {
        let mut tracker = ToolCallTracker::new();
        tracker.add("call.1".into(), "bash".into(), json!({"cmd": "ls"}));
        tracker.start("call.1");
        tracker.complete("call.1", "files".into(), None);
        let state = tracker.get("call.1").unwrap();
        assert!(matches!(state, ToolCallState::Completed { .. }));
    }

    #[test]
    fn tracker_fail_flow() {
        let mut tracker = ToolCallTracker::new();
        tracker.add("call.1".into(), "bash".into(), json!({"cmd": "bad"}));
        tracker.start("call.1");
        tracker.fail("call.1", "exit code 1".into());
        let state = tracker.get("call.1").unwrap();
        assert!(matches!(state, ToolCallState::Error { .. }));
    }
}
