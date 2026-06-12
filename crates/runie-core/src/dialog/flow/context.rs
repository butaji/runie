//! Flow Context and Results

use crate::commands::CommandResult;
use std::collections::HashMap;

/// Flow context - shared state across steps
#[derive(Debug, Clone, Default)]
pub struct FlowContext {
    /// Current step index
    pub step: usize,
    /// Arbitrary data keyed by string
    pub data: HashMap<String, String>,
    /// Error message if validation failed
    pub error: Option<String>,
    /// Whether flow is complete
    pub done: bool,
}

impl FlowContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_data(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.data.insert(key.into(), value.into());
        self
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.data.get(key).map(|s| s.as_str())
    }

    pub fn set_error(&mut self, msg: impl Into<String>) {
        self.error = Some(msg.into());
    }

    pub fn clear_error(&mut self) {
        self.error = None;
    }
}

/// Result type for flow operations
#[derive(Debug, Clone, PartialEq)]
pub enum FlowResult {
    /// Continue to next step
    Next,
    /// Go to previous step
    Prev,
    /// Jump to step by index
    Jump(usize),
    /// Jump to branch
    Branch(String),
    /// Exit flow with result
    Done(CommandResult),
    /// Show error and stay
    Error(String),
}

impl FlowResult {
    pub fn next() -> Self {
        Self::Next
    }
    pub fn prev() -> Self {
        Self::Prev
    }
    pub fn jump(i: usize) -> Self {
        Self::Jump(i)
    }
    pub fn branch(name: impl Into<String>) -> Self {
        Self::Branch(name.into())
    }
    pub fn done(result: CommandResult) -> Self {
        Self::Done(result)
    }
    pub fn error(msg: impl Into<String>) -> Self {
        Self::Error(msg.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flow_context() {
        let ctx = FlowContext::new()
            .with_data("name", "Alice")
            .with_data("age", "30");

        assert_eq!(ctx.get("name"), Some("Alice"));
        assert_eq!(ctx.get("age"), Some("30"));
        assert_eq!(ctx.get("missing"), None);
    }

    #[test]
    fn test_flow_result() {
        assert!(matches!(FlowResult::next(), FlowResult::Next));
        assert!(matches!(FlowResult::prev(), FlowResult::Prev));
        assert!(matches!(FlowResult::jump(5), FlowResult::Jump(5)));
        assert!(matches!(
            FlowResult::done(CommandResult::None),
            FlowResult::Done(_)
        ));
        assert!(matches!(FlowResult::error("fail"), FlowResult::Error(msg) if msg == "fail"));
    }
}
