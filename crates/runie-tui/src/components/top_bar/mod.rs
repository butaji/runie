use crate::tui::state::TopBarState;

pub mod builder;
pub mod gauge;
pub mod helpers;
pub mod render;

#[cfg(test)]
mod tests;

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
}

impl TopBarViewModel {
    pub fn from_state(state: &TopBarState, agent_running: bool, braille_frame: usize) -> Self {
        Self {
            repo: state.repo.clone(),
            branch: state.branch.clone(),
            path: state.path.clone(),
            context_window: state.context_window.unwrap_or(128_000),
            estimated_tokens: state.estimated_tokens.unwrap_or(0),
            agent_running,
            braille_frame,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_state_with_defaults() {
        let state = TopBarState::default();
        let vm = TopBarViewModel::from_state(&state, false, 0);
        assert_eq!(vm.repo, "");
        assert_eq!(vm.branch, "");
        assert_eq!(vm.path, "");
        assert_eq!(vm.context_window, 128_000);
        assert_eq!(vm.estimated_tokens, 0);
        assert_eq!(vm.agent_running, false);
        assert_eq!(vm.braille_frame, 0);
    }

    #[test]
    fn test_from_state_with_values() {
        let mut state = TopBarState::default();
        state.repo = "runie".to_string();
        state.branch = "main".to_string();
        state.path = "src/main.rs".to_string();
        state.context_window = Some(200_000);
        state.estimated_tokens = Some(40_000);

        let vm = TopBarViewModel::from_state(&state, true, 5);
        assert_eq!(vm.repo, "runie");
        assert_eq!(vm.branch, "main");
        assert_eq!(vm.path, "src/main.rs");
        assert_eq!(vm.context_window, 200_000);
        assert_eq!(vm.estimated_tokens, 40_000);
        assert_eq!(vm.agent_running, true);
        assert_eq!(vm.braille_frame, 5);
    }

    #[test]
    fn test_from_state_partial() {
        let mut state = TopBarState::default();
        state.context_window = Some(200_000);
        state.estimated_tokens = None;

        let vm = TopBarViewModel::from_state(&state, false, 0);
        assert_eq!(vm.context_window, 200_000);
        assert_eq!(vm.estimated_tokens, 0);
    }
}
