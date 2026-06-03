use crate::tui::state::{TopBarState, TuiMode};

pub mod builder;
pub mod gauge;
pub mod helpers;
pub mod render;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod helpers_test;

pub use gauge::{calculate_pct, draw_gauge, format_context_window, format_token_count};
pub use render::render_top_bar;
pub use helpers::{shorten_path, build_left_spans};

#[derive(Clone)]
pub struct TopBarViewModel {
    pub repo: String,
    pub branch: String,
    pub path: String,
    pub context_window: usize,
    pub estimated_tokens: usize,
    pub agent_running: bool,
    pub braille_frame: usize,
    pub mode: TuiMode,
}

impl TopBarViewModel {
    pub fn from_state(state: &TopBarState, agent_running: bool, braille_frame: usize, mode: TuiMode) -> Self {
        Self {
            repo: state.repo.clone(),
            branch: state.branch.clone(),
            path: state.path.clone(),
            context_window: state.context_window.unwrap_or(128_000),
            estimated_tokens: state.estimated_tokens.unwrap_or(0),
            agent_running,
            braille_frame,
            mode,
        }
    }
}
