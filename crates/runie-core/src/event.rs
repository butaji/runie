//! Centralized Event Types
//!
//! All events in the application flow through a single channel.

use serde::{Deserialize, Serialize};

/// All events in the application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    // === UI Events ===
    Input(char),
    Backspace,
    Submit,
    ScrollUp,
    ScrollDown,
    
    // === System Events ===
    Quit,
    Reset,
    
    // === Agent Events (with composite ID like "req.1") ===
    AgentThinking { id: String },
    AgentResponse { id: String, content: String },
    AgentToolStart { id: String, name: String },
    AgentToolEnd { id: String, name: String, output: String },
    AgentTurnComplete { id: String, duration_secs: f64 },
    AgentDone { id: String },
    AgentError { id: String, message: String },
    
    // === Internal Events ===
    SpawnAgent,
}

impl Event {
    /// Check if this event should cause a redraw
    pub fn needs_redraw(&self) -> bool {
        matches!(
            self,
            Event::Input(_)
                | Event::Backspace
                | Event::AgentResponse { .. }
                | Event::AgentThinking { .. }
                | Event::AgentToolStart { .. }
                | Event::AgentToolEnd { .. }
                | Event::AgentTurnComplete { .. }
                | Event::AgentDone { .. }
                | Event::Reset
        )
    }
}
