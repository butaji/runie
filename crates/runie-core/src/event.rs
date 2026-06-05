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
    
    // === Agent Events (with composite ID like \"req.1\") ===
    AgentThinking { id: String },
    AgentResponse { id: String, content: String },
    AgentDone { id: String },
    AgentError { id: String, message: String },
    
    // === Internal Events ===
    SpawnAgent,  // Signal to spawn agent for next queued request
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
                | Event::AgentDone { .. }
                | Event::Reset
        )
    }
}
