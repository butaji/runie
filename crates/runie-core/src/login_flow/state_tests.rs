//! Pure state-machine tests for the login flow.

#[cfg(test)]
mod tests {
    use crate::login_flow::state::{LoginFlowState, LoginStep};

    #[test]
    fn login_flow_starts_at_provider_picker() {
        let flow = LoginFlowState::new();
        assert_eq!(flow.step, LoginStep::ProviderPicker);
        assert!(flow.provider.is_empty());
        assert!(flow.key.is_empty());
        assert!(flow.available_models.is_empty());
        assert!(flow.selected_models.is_empty());
    }

    #[test]
    fn login_flow_select_provider() {
        let flow = LoginFlowState::new().with_provider("minimax".into());
        assert_eq!(flow.step, LoginStep::KeyInput);
        assert_eq!(flow.provider, "minimax");
    }

    #[test]
    fn login_flow_submit_key_goes_to_validating() {
        let flow = LoginFlowState::new()
            .with_provider("minimax".into())
            .with_key("sk-test".into());
        assert_eq!(flow.step, LoginStep::Validating);
        assert_eq!(flow.key, "sk-test");
        assert!(flow.available_models.is_empty());
        assert!(flow.selected_models.is_empty());
        assert!(!flow.validated);
    }

    #[test]
    fn login_flow_validation_success_reaches_model_select() {
        let flow = LoginFlowState::new()
            .with_provider("minimax".into())
            .with_key("sk-test".into())
            .with_validation_success(vec!["MiniMax-M3".into(), "MiniMax-M2.7".into()]);
        assert_eq!(flow.step, LoginStep::ModelSelect);
        assert_eq!(flow.available_models.len(), 2);
        assert!(flow.selected_models.contains("MiniMax-M3"));
        assert!(flow.selected_models.contains("MiniMax-M2.7"));
        assert!(flow.validated);
    }

    #[test]
    fn login_flow_validation_success_with_empty_models() {
        let flow = LoginFlowState::new()
            .with_provider("unknown".into())
            .with_key("k".into())
            .with_validation_success(vec![]);
        assert_eq!(flow.step, LoginStep::ModelSelect);
        assert!(flow.available_models.is_empty());
        assert!(flow.selected_models.is_empty());
    }

    #[test]
    fn login_flow_toggle_model() {
        let mut flow = LoginFlowState::new()
            .with_provider("minimax".into())
            .with_key("sk-test".into())
            .with_validation_success(vec!["MiniMax-M3".into(), "MiniMax-M2.7".into()]);
        assert!(flow.selected_models.contains("MiniMax-M3"));
        flow.toggle_model("MiniMax-M3");
        assert!(!flow.selected_models.contains("MiniMax-M3"));
        flow.toggle_model("MiniMax-M3");
        assert!(flow.selected_models.contains("MiniMax-M3"));
    }

    #[test]
    fn login_flow_is_done() {
        let mut flow = LoginFlowState::new();
        assert!(!flow.is_done());
        flow.step = LoginStep::Done;
        assert!(flow.is_done());
    }

    #[test]
    fn fetched_models_replaces_list_and_selects_new() {
        let flow = LoginFlowState::new()
            .with_provider("minimax".into())
            .with_key("k".into())
            .with_validation_success(vec!["A".into(), "B".into()]);
        let flow = flow.with_fetched_models(vec!["A".into(), "B".into(), "C".into()]);
        assert_eq!(flow.available_models, vec!["A", "B", "C"]);
        assert!(flow.selected_models.contains("A"));
        assert!(flow.selected_models.contains("B"));
        assert!(flow.selected_models.contains("C"));
    }

    #[test]
    fn fetched_models_preserves_user_toggle_on_existing() {
        let mut flow = LoginFlowState::new()
            .with_provider("minimax".into())
            .with_key("k".into())
            .with_validation_success(vec!["A".into(), "B".into()]);
        flow.toggle_model("A");
        let flow = flow.with_fetched_models(vec!["A".into(), "B".into()]);
        assert!(!flow.selected_models.contains("A"));
        assert!(flow.selected_models.contains("B"));
    }

    #[test]
    fn fetched_models_drops_models_no_longer_returned() {
        let flow = LoginFlowState::new()
            .with_provider("minimax".into())
            .with_key("k".into())
            .with_validation_success(vec!["A".into(), "B".into()]);
        let flow = flow.with_fetched_models(vec!["A".into()]);
        assert_eq!(flow.available_models, vec!["A"]);
        assert!(flow.selected_models.contains("A"));
        assert!(!flow.selected_models.contains("B"));
    }

    #[test]
    fn fetched_models_empty_list_clears_selection() {
        let flow = LoginFlowState::new()
            .with_provider("minimax".into())
            .with_key("k".into())
            .with_validation_success(vec!["A".into()]);
        let flow = flow.with_fetched_models(vec![]);
        assert!(flow.available_models.is_empty());
        assert!(flow.selected_models.is_empty());
    }

    #[test]
    fn fetched_models_disjoint_list() {
        let flow = LoginFlowState::new()
            .with_provider("minimax".into())
            .with_key("k".into())
            .with_validation_success(vec!["A".into()]);
        let flow = flow.with_fetched_models(vec!["X".into(), "Y".into()]);
        assert_eq!(flow.available_models, vec!["X", "Y"]);
        assert!(flow.selected_models.contains("X"));
        assert!(flow.selected_models.contains("Y"));
    }
}
