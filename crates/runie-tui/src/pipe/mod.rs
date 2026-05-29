//! Pipe architecture for unidirectional data flow.
//!
//! ```text
//! InputPipe → StatePipe → ViewModelPipe → RenderPipe
//!     │           │              │            │
//!     ↓           ↓              ↓            ↓
//!  Events    StateChange     ViewModels   Terminal
//! ```

mod input;
mod render;
mod state;
mod view_model;

pub use input::{InputMsg, InputPipe};
pub use state::StatePipe;
pub use view_model::ViewModelPipe;
pub use render::RenderPipe;

// Re-export Cmd from tui for convenience in StateChange
pub use crate::tui::Cmd;

/// Pipe trait — unidirectional transformation
pub trait Pipe<Input> {
    type Output;
    fn pipe(&self, input: Input) -> Self::Output;
}

/// StateChange represents state mutations + side effects
#[derive(Debug, Clone, Default)]
pub struct StateChange {
    pub cmds: Vec<Cmd>,
    pub needs_render: bool,
}

impl StateChange {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn with_cmd(mut self, cmd: Cmd) -> Self {
        self.cmds.push(cmd);
        self
    }

    pub fn needs_render(mut self) -> Self {
        self.needs_render = true;
        self
    }
}