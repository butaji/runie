use crate::tui::state::{AppState, Msg};
use super::{Pipe, StateChange};

/// StatePipe is the application reducer.
/// Receives Msg and produces StateChange effects.
pub struct StatePipe {
    state: AppState,
}

impl StatePipe {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    pub fn state(&self) -> &AppState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut AppState {
        &mut self.state
    }

    /// Process a message — this will eventually call the existing update functions
    pub fn process(&mut self, msg: Msg) -> StateChange {
        // TODO: integrate with existing update::update() in Phase 3
        StateChange::none().needs_render()
    }
}

impl Pipe<Msg> for StatePipe {
    type Output = StateChange;

    fn pipe(&self, _msg: Msg) -> StateChange {
        // TODO: implement in Phase 3
        StateChange::none().needs_render()
    }
}