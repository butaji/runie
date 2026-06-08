#[derive(Debug, Clone)]
pub enum AgentEvent {
    Thinking { id: String },
    ThoughtDone { id: String },
    ToolStart { id: String, name: String },
    ToolEnd { duration_secs: f64, output: String },
    Response { id: String, content: String },
    TurnComplete { id: String, duration_secs: f64 },
    Done { id: String },
    Error { id: String, message: String },
}

impl AgentEvent {
    pub fn to_core_event(&self) -> runie_core::Event {
        match self.clone() {
            AgentEvent::Thinking { id } => runie_core::Event::AgentThinking { id },
            AgentEvent::ThoughtDone { id } => runie_core::Event::AgentThoughtDone { id },
            AgentEvent::ToolStart { id, name } => {
                runie_core::Event::AgentToolStart { id, name }
            }
            AgentEvent::ToolEnd { duration_secs, output } => {
                runie_core::Event::AgentToolEnd { duration_secs, output }
            }
            AgentEvent::Response { id, content } => {
                runie_core::Event::AgentResponse { id, content }
            }
            AgentEvent::TurnComplete { id, duration_secs } => {
                runie_core::Event::AgentTurnComplete { id, duration_secs }
            }
            AgentEvent::Done { id } => runie_core::Event::AgentDone { id },
            AgentEvent::Error { id, message } => {
                runie_core::Event::AgentError { id, message }
            }
        }
    }
}
