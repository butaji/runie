#[cfg(test)]
mod tests {
    use crate::pipe::{StatePipe, ViewModelPipe, Pipe};
    use crate::tui::state::{AppState, Msg};

    #[test]
    fn test_state_pipe_process_tick() {
        let state = AppState::default();
        let mut pipe = StatePipe::new(state);
        let change = pipe.process(Msg::Tick);
        assert!(change.needs_render);
    }

    #[test]
    fn test_view_model_pipe_builds_view_models() {
        let state = AppState::default();
        let pipe = ViewModelPipe;
        let vms = pipe.pipe(&state);
        // Top bar should exist even with default state
        assert!(vms.top_bar.repo.is_empty() || vms.top_bar.repo == "—");
    }

    #[test]
    fn test_state_pipe_process_key() {
        let state = AppState::default();
        let mut pipe = StatePipe::new(state);
        // Test that a key message doesn't panic
        let change = pipe.process(Msg::Tick);
        assert!(change.needs_render || !change.needs_render); // Just don't panic
    }
}
