//! Centralized Event Types

#[derive(Debug, Clone)]
pub enum Event {

    Input(char),
    Backspace,
    Submit,
    ScrollUp,
    ScrollDown,
    

    Quit,
    Reset,
    

    AgentThinking { id: String },
    AgentThoughtDone { id: String },
    AgentToolStart { id: String, name: String },
    AgentToolEnd { duration_secs: f64, output: String },
    AgentResponse { id: String, content: String },
    AgentTurnComplete { id: String, duration_secs: f64 },
    AgentDone { id: String },
    AgentError { id: String, message: String },
    

    SwitchModel { provider: String, model: String },
    FollowUp,
    Abort,


    SpawnAgent,
    ToggleCollapse { index: usize },
    ToggleThought,
    ToggleTool,
}
