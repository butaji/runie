//! Pipe modules — kept for organization only (Pipe trait removed).

mod input_msg;
mod render;
mod view_model;

#[cfg(test)]
mod tests;

pub use input_msg::InputMsg;
pub use view_model::ViewModelPipe;
pub use render::RenderPipe;

// Re-export Cmd from tui for convenience in StateChange
pub use crate::tui::Cmd;

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