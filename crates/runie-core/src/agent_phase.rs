//! Agent phase tracking for status indicator.
//!
//! Tracks the current agent phase (thinking, composing, tool, waiting) and
//! provides formatting for elapsed time display.

use std::fmt;
use std::time::Instant;

/// Agent execution phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentPhase {
    /// Agent is thinking/reasoning.
    Thinking,
    /// Agent is composing a response (no tools called yet).
    Composing,
    /// Agent is running a tool.
    Tool { name: &'static str },
    /// Agent is waiting for user input.
    Waiting,
    /// Agent is idle (no active turn).
    Idle,
}

impl fmt::Display for AgentPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentPhase::Thinking => write!(f, "thinking"),
            AgentPhase::Composing => write!(f, "composing"),
            AgentPhase::Tool { name } => write!(f, "tool:{}", name),
            AgentPhase::Waiting => write!(f, "waiting"),
            AgentPhase::Idle => write!(f, "idle"),
        }
    }
}

impl AgentPhase {
    /// Short display label for status bar.
    pub fn label(&self) -> &'static str {
        match self {
            AgentPhase::Thinking => "thinking",
            AgentPhase::Composing => "composing",
            AgentPhase::Tool { name } => name,
            AgentPhase::Waiting => "waiting",
            AgentPhase::Idle => "",
        }
    }

    /// Whether this phase should show elapsed time.
    pub fn show_elapsed(&self) -> bool {
        !matches!(self, AgentPhase::Idle | AgentPhase::Waiting)
    }
}

/// Format elapsed duration as `12.3s` or `1m5s`.
pub fn format_elapsed(started: Option<Instant>) -> String {
    match started {
        Some(start) => {
            let elapsed = started.map(|s| s.elapsed()).unwrap_or_default();
            let secs = elapsed.as_secs();
            if secs >= 60 {
                let mins = secs / 60;
                let secs_rem = secs % 60;
                format!("{}m{}s", mins, secs_rem)
            } else {
                format!("{:.1}s", elapsed.as_secs_f64())
            }
        }
        None => String::new(),
    }
}

/// Elapsed seconds as a float (for display).
pub fn elapsed_secs(started: Option<Instant>) -> f64 {
    started.map(|s| s.elapsed().as_secs_f64()).unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_elapsed_under_minute() {
        let start = Instant::now() - std::time::Duration::from_millis(12300);
        let formatted = format_elapsed(Some(start));
        assert!(formatted.ends_with('s'), "should end with 's': {}", formatted);
    }

    #[test]
    fn format_elapsed_over_minute() {
        let start = Instant::now() - std::time::Duration::from_secs(65);
        let formatted = format_elapsed(Some(start));
        assert!(formatted.contains('m'), "should contain 'm': {}", formatted);
        assert!(formatted.contains('s'), "should contain 's': {}", formatted);
    }

    #[test]
    fn format_elapsed_none_returns_empty() {
        assert_eq!(format_elapsed(None), "");
    }

    #[test]
    fn agent_phase_display() {
        assert_eq!(AgentPhase::Thinking.to_string(), "thinking");
        assert_eq!(AgentPhase::Composing.to_string(), "composing");
        assert_eq!(AgentPhase::Tool { name: "bash" }.to_string(), "tool:bash");
        assert_eq!(AgentPhase::Waiting.to_string(), "waiting");
        assert_eq!(AgentPhase::Idle.to_string(), "idle");
    }

    #[test]
    fn agent_phase_label() {
        assert_eq!(AgentPhase::Thinking.label(), "thinking");
        assert_eq!(AgentPhase::Tool { name: "bash" }.label(), "bash");
    }

    #[test]
    fn agent_phase_show_elapsed() {
        assert!(AgentPhase::Thinking.show_elapsed());
        assert!(AgentPhase::Tool { name: "bash" }.show_elapsed());
        assert!(!AgentPhase::Waiting.show_elapsed());
        assert!(!AgentPhase::Idle.show_elapsed());
    }
}
