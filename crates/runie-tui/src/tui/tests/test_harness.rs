use crate::tui::state::{AppState, Cmd};
use crate::tui::update::misc::handle_submit;
use crate::tui::update::agent::handle_agent_event;
use crate::components::MessageItem;
use runie_agent::{AgentEvent, AgentMessage, ContentPart};
use std::time::Instant;

/// Test harness for TUI agent tests
pub struct AgentTestHarness {
    pub state: AppState,
    pub messages: Vec<AgentEvent>,
    pub start_time: Instant,
}

impl AgentTestHarness {
    pub fn new() -> Self {
        let mut state = AppState::default();
        state.current_model = Some("test-model".to_string());
        Self {
            state,
            messages: Vec::new(),
            start_time: Instant::now(),
        }
    }
    
    /// Simulate user submitting a message
    pub fn submit_user_message(&mut self, text: &str) -> Vec<Cmd> {
        self.state.textarea.insert_str(text);
        handle_submit(&mut self.state)
    }
    
    /// Simulate agent event arriving
    pub fn handle_agent_event(&mut self, event: AgentEvent) {
        handle_agent_event(&mut self.state, event)
    }
    
    /// Assert state conditions
    pub fn assert_agent_running(&self) {
        assert!(self.state.agent_running, "agent should be running");
    }
    
    pub fn assert_has_assistant_placeholder(&self) {
        assert!(
            self.state.messages.iter().any(|m| matches!(m, MessageItem::Assistant { text, .. } if text.is_empty())),
            "should have empty assistant placeholder"
        );
    }
    
    pub fn assert_last_assistant_text(&self, expected: &str) {
        let last = self.state.messages.iter().rev()
            .find_map(|m| match m { MessageItem::Assistant { text, .. } => Some(text.as_str()), _ => None });
        assert_eq!(last, Some(expected), "last assistant text mismatch");
    }
}

/// Builder for test scenarios
pub struct ScenarioBuilder {
    harness: AgentTestHarness,
}

impl ScenarioBuilder {
    pub fn new() -> Self {
        Self { harness: AgentTestHarness::new() }
    }
    
    pub fn with_model(mut self, model: &str) -> Self {
        self.harness.state.current_model = Some(model.to_string());
        self
    }
    
    pub fn user_says(mut self, text: &str) -> Self {
        self.harness.submit_user_message(text);
        self
    }
    
    pub fn agent_stream_text(mut self, text: &str) -> Self {
        let event = AgentEvent::MessageUpdate {
            message: AgentMessage { 
                role: "assistant".to_string(),
                content: vec![ContentPart::Text { text: text.to_string() }],
                timestamp: 0,
                usage: None,
                stop_reason: None,
                error_message: None,
                tool_calls: vec![],
            },
                        delta: String::new(),
            replace: true,turn: 1,
            delta: text.to_string(),
        };
        self.harness.handle_agent_event(event);
        self
    }
    
    pub fn build(self) -> AgentTestHarness {
        self.harness
    }
}
