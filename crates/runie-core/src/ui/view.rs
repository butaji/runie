//! View layer — Declarative UI composition
//!
//! Usage:
//!   let view = View::new(state)
//!       .panel(ChatPanel)
//!       .panel(StatusPanel.when(state.turn_active))
//!       .panel(InputPanel);
//!
//! Each Panel is a declarative description of what to render.
//! No rendering logic here — that's in the render layer.

use crate::model::AppState;

/// Declarative view — what panels to show
pub struct View<'a> {
    pub state: &'a AppState,
    pub panels: Vec<Panel>,
}

/// Panel kinds — declarative, not imperative
#[derive(Clone, Copy, Debug)]
pub enum Panel {
    Chat,
    Status { show: bool },
    Input,
}

impl<'a> View<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self {
            state,
            panels: vec![
                Panel::Chat,
                Panel::Status { show: state.turn_active },
                Panel::Input,
            ],
        }
    }

    pub fn panel(mut self, panel: Panel) -> Self {
        self.panels.push(panel);
        self
    }
}
