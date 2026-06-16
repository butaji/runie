//! Test DSL — fluent builder for driving AppState in tests.
//!
//! ```ignore
//! let mut state = AppState::default();
//! state.type_text("hello").submit();
//! state.agent("req.0").think().respond("hi").complete(1.0).done();
//! ```

use crate::event::{AgentEvent, Event, InputEvent};
use crate::model::AppState;

/// Fluent builder for an agent turn sequence.
pub struct AgentTurn<'a> {
    state: &'a mut AppState,
    id: String,
}

impl<'a> AgentTurn<'a> {
    pub fn new(state: &'a mut AppState, id: impl Into<String>) -> Self {
        Self {
            state,
            id: id.into(),
        }
    }

    pub fn think(self) -> Self {
        self.state.update(Event::Agent(AgentEvent::Thinking {
            id: self.id.clone(),
        }));
        self
    }

    pub fn respond(self, content: impl Into<String>) -> Self {
        self.state.update(Event::Agent(AgentEvent::Response {
            id: self.id.clone(),
            content: content.into(),
        }));
        self
    }

    pub fn thought_done(self) -> Self {
        self.state.update(Event::Agent(AgentEvent::ThoughtDone {
            id: self.id.clone(),
        }));
        self
    }

    pub fn tool(self, name: impl Into<String>, output: impl Into<String>) -> Self {
        let name = name.into();
        self.state.update(Event::Agent(AgentEvent::ToolStart {
            id: self.id.clone(),
            name,
        }));
        self.state.update(Event::Agent(AgentEvent::ToolEnd {
            duration_secs: 0.5,
            output: output.into(),
        }));
        self
    }

    pub fn tool_start(self, name: impl Into<String>) -> Self {
        self.state.update(Event::Agent(AgentEvent::ToolStart {
            id: self.id.clone(),
            name: name.into(),
        }));
        self
    }

    pub fn complete(self, duration_secs: f64) -> Self {
        self.state.update(Event::Agent(AgentEvent::TurnComplete {
            id: self.id.clone(),
            duration_secs,
        }));
        self
    }

    pub fn done(self) {
        self.state.update(Event::Agent(AgentEvent::Done { id: self.id }));
    }

    pub fn error(self, message: impl Into<String>) {
        self.state.update(Event::Agent(AgentEvent::Error {
            id: self.id,
            message: message.into(),
        }));
    }
}

/// Extension trait adding DSL methods to AppState.
pub trait AppStateDsl {
    /// Type a string character-by-character into the input buffer.
    fn type_text(&mut self, text: &str) -> &mut Self;

    /// Submit the current input.
    fn submit(&mut self) -> &mut Self;

    /// Start an agent turn builder.
    fn agent(&mut self, id: impl Into<String>) -> AgentTurn<'_>;
}

impl AppStateDsl for AppState {
    fn type_text(&mut self, text: &str) -> &mut Self {
        for c in text.chars() {
            self.update(Event::Input(InputEvent::Input(c)));
        }
        self
    }

    fn submit(&mut self) -> &mut Self {
        self.update(Event::Input(InputEvent::Submit));
        self
    }

    fn agent(&mut self, id: impl Into<String>) -> AgentTurn<'_> {
        AgentTurn::new(self, id)
    }
}
