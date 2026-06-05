//! Centralized Event Types
//!
//! All events in the application flow through a single channel.
//! This includes UI events, agent events, and system events.

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
    
    // === Agent Events ===
    AgentResponse { content: String },
    AgentThinking,
    AgentDone,
    AgentError { message: String },
    
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
                | Event::AgentThinking
                | Event::AgentDone
                | Event::Reset
        )
    }
}
