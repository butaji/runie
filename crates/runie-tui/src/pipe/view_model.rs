use crate::tui::state::AppState;
use crate::tui::view_models::ViewModels;
use super::Pipe;

/// ViewModelPipe transforms AppState into ViewModels using builders.
pub struct ViewModelPipe;

impl ViewModelPipe {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ViewModelPipe {
    fn default() -> Self {
        Self::new()
    }
}

impl Pipe<&AppState> for ViewModelPipe {
    type Output = ViewModels;

    fn pipe(&self, state: &AppState) -> ViewModels {
        // TODO: use builders in Phase 3
        // For now, delegate to existing ViewModels::from_render_state
        // by first converting AppState -> RenderState
        use crate::tui::state::RenderState;
        let render_state = RenderState::from(state);
        use crate::components::command_palette::CommandPalette;
        use crate::components::message_list::render::WrapCache;
        let palette = CommandPalette::new();
        let wrap_cache = WrapCache::new();
        ViewModels::from_render_state(&render_state, &palette, wrap_cache)
    }
}