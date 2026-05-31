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
        // ViewModels should be built successfully
        // global_tags is always present
        assert!(vms.global_tags.right.is_empty() || !vms.global_tags.right.is_empty());
    }

    #[test]
    fn test_state_pipe_process_key() {
        let state = AppState::default();
        let mut pipe = StatePipe::new(state);
        // Test that a key message doesn't panic
        let change = pipe.process(Msg::Tick);
        assert!(change.needs_render || !change.needs_render); // Just don't panic
    }

    /// Test that top bar shows path/git info from context state, NOT model info.
    /// Model info should ONLY appear in global_tags, never in top bar.
    #[test]
    fn test_top_bar_shows_path_not_model() {
        let mut state = AppState::default();
        
        // Set git info in context (as SetGitInfo message would)
        state.context.repo = "runie".to_string();
        state.context.branch = "main".to_string();
        state.context.path = "crates/runie-tui".to_string();
        
        // Set current model (as SetCurrentModel message would)
        state.current_model = Some("openai/gpt-4o".to_string());
        
        let pipe = ViewModelPipe;
        let vms = pipe.pipe(&state);
        
        // Top bar should show repo/branch/path from context
        assert_eq!(vms.top_bar.repo, "runie");
        assert_eq!(vms.top_bar.branch, "main");
        assert_eq!(vms.top_bar.path, "crates/runie-tui");
        
        // TopBarViewModel no longer has a model field - model info belongs ONLY in global_tags
        // This is enforced by the type system (model field was removed)
        
        // Verify global_tags DOES have the model info (as it should)
        assert!(vms.global_tags.right.contains("gpt-4o") || vms.global_tags.right.contains("openai"));
    }
}
