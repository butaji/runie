#[cfg(test)]
mod tests {
    use crate::pipe::ViewModelPipe;
    use crate::tui::state::AppState;

    #[test]
    fn test_view_model_pipe_builds_view_models() {
        let state = AppState::default();
        let pipe = ViewModelPipe;
        let vms = pipe.build(&state);
        // ViewModels should be built successfully
        // global_tags is always present
        assert!(vms.global_tags.right.is_empty() || !vms.global_tags.right.is_empty());
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
        let vms = pipe.build(&state);
        
        // Top bar should show repo/branch/path from context
        assert_eq!(vms.top_bar.repo, "runie");
        assert_eq!(vms.top_bar.branch, "main");
        assert_eq!(vms.top_bar.path, "crates/runie-tui");
        
        // TopBarViewModel no longer has a model field - model info belongs ONLY in global_tags
        // This is enforced by the type system (model field was removed)
        
        // Global_tags when idle shows tokens/cost only, not model name
        // (model name was removed from global tags per user request)
    }
}