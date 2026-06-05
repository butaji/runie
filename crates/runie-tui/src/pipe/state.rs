use crate::components::CommandPalette;
use crate::tui::state::{AppState, Msg};
use crate::tui::update;
use super::StateChange;

/// StatePipe is the application reducer.
/// Receives Msg and produces StateChange effects.
pub struct StatePipe {
    state: AppState,
    palette: CommandPalette,
}

impl StatePipe {

    #[must_use]
    pub fn new(state: AppState) -> Self {
        Self {
            state,
            palette: CommandPalette::new(),
        }
    }

    pub fn state(&self) -> &AppState {
        &self.state
    }

    pub fn palette(&self) -> &CommandPalette {
        &self.palette
    }

    /// Process a message — wraps existing update::update()
    pub fn process(&mut self, msg: Msg) -> StateChange {
        let cmds = update(&mut self.state, &mut self.palette, msg);

        // Always render after update since state may have changed
        StateChange {
            cmds,
            needs_render: true,
        }
    }
}