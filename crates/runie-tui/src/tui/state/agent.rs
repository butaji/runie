use crate::components::status_bar::BackgroundJob;
use runie_ai::TokenUsage;

/// AgentState contains agent execution state.
#[derive(Clone)]
pub struct AgentState {
    pub agent_running: bool,
    pub current_model: Option<String>,
    pub token_usage: TokenUsage,
    pub session_token_usage: TokenUsage,
    pub agent_start_time: Option<std::time::Instant>,
    pub background_jobs: Vec<BackgroundJob>,
    pub thinking_start: Option<std::time::Instant>,
    pub thinking_duration: Option<std::time::Duration>,
    pub is_thinking: bool,
}

impl Default for AgentState {
    fn default() -> Self {
        Self {
            agent_running: false,
            current_model: None,
            token_usage: TokenUsage::default(),
            session_token_usage: TokenUsage::default(),
            agent_start_time: None,
            background_jobs: Vec::new(),
            thinking_start: None,
            thinking_duration: None,
            is_thinking: false,
        }
    }
}
