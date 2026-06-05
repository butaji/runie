//! Centralized Event Types

use serde::{Deserialize, Serialize};

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
    AgentThinking { id: String },
    AgentThoughtDone { id: String },
    AgentToolDone { id: String, name: String, duration_secs: f64 },
    AgentResponse { id: String, content: String },
    AgentTurnComplete { id: String, duration_secs: f64 },
    AgentDone { id: String },
    AgentError { id: String, message: String },
    
    // === Internal Events ===
    SpawnAgent,
}

impl Event {
    pub fn needs_redraw(&self) -> bool {
        matches!(
            self,
            Event::Input(_)
                | Event::Backspace
                | Event::AgentResponse { .. }
                | Event::AgentThinking { .. }
                | Event::AgentThoughtDone { .. }
                | Event::AgentToolDone { .. }
                | Event::AgentTurnComplete { .. }
                | Event::AgentDone { .. }
                | Event::Reset
        )
    }
}
