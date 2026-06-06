//! Centralized Event Types

#[derive(Debug, Clone)]
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
    AgentToolStart { id: String, name: String },
    AgentToolEnd { duration_secs: f64 },
    AgentResponse { id: String, content: String },
    AgentTurnComplete { id: String, duration_secs: f64 },
    AgentDone { id: String },
    AgentError { id: String, message: String },
    
    // === Model Switching ===
    SwitchModel { provider: String, model: String },
    ShowHelp,
    
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
                | Event::AgentToolStart { .. }
                | Event::AgentToolEnd { .. }
                | Event::AgentTurnComplete { .. }
                | Event::AgentDone { .. }
                | Event::Reset
                | Event::SwitchModel { .. }
                | Event::ShowHelp
        )
    }
}
