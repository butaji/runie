use crate::components::CommandPalette;
use crate::tui::state::{AppState, Msg};
use crate::tui::update;
use super::{Pipe, StateChange};

/// StatePipe is the application reducer.
/// Receives Msg and produces StateChange effects.
pub struct StatePipe {
    state: AppState,
    palette: CommandPalette,
}

impl StatePipe {
    pub fn new(state: AppState) -> Self {
        Self {
            state,
            palette: CommandPalette::new(),
        }
    }

    pub fn state(&self) -> &AppState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut AppState {
        &mut self.state
    }

    pub fn palette(&self) -> &CommandPalette {
        &self.palette
    }

    pub fn palette_mut(&mut self) -> &mut CommandPalette {
        &mut self.palette
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

impl Pipe<Msg> for StatePipe {
    type Output = StateChange;

    fn pipe(&self, _msg: Msg) -> StateChange {
        // StatePipe::process() requires &mut self, use it directly from main loop
        // This is a convenience shim that discards state mutations
        StateChange::none()
    }
}