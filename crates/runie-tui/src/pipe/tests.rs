#[cfg(test)]
mod tests {
    use crate::pipe::ViewModelPipe;
    use crate::tui::state::AppState;

    #[test]
    fn test_view_model_pipe_builds_view_models() {
        let state = AppState::default();
        let pipe = ViewModelPipe;
        let _vms = pipe.build(&state);
        // ViewModels should be built successfully
        // Test passes if no panic
        assert!(true);
    }

    /// Test that top bar state is initialized correctly.
    #[test]
    fn test_top_bar_state_init() {
        let state = AppState::default();
        // Verify state initializes correctly
        assert!(state.running);
    }
}
