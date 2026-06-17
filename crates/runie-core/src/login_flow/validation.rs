//! Scenario / flow validation tests for the non-blocking login flow.
//!
//! In every scenario the user lands on the model selector **immediately**
//! after submitting the key. The network call is a best-effort enrichment.

#[cfg(test)]
mod tests {
    use crate::event::{Event, LoginFlowEvent};
    use crate::login_flow::state::{LoginFlowState, LoginStep};
    use crate::model::AppState;

    // -----------------------------------------------------------------------
    // Layer 1: pure state transitions
    // -----------------------------------------------------------------------

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
    fn login_flow_submit_key_goes_straight_to_model_select() {
        let flow = LoginFlowState::new()
            .with_provider("minimax".into())
            .with_key_and_defaults(
                "sk-test".into(),
                vec!["MiniMax-M3".into(), "MiniMax-M2.7".into()],
            );
        assert_eq!(flow.step, LoginStep::ModelSelect);
        assert_eq!(flow.key, "sk-test");
        assert_eq!(flow.available_models.len(), 2);
        assert!(flow.selected_models.contains("MiniMax-M3"));
        assert!(flow.selected_models.contains("MiniMax-M2.7"));
    }

    #[test]
    fn login_flow_submit_key_with_empty_defaults() {
        let flow = LoginFlowState::new()
            .with_provider("unknown".into())
            .with_key_and_defaults("k".into(), vec![]);
        assert_eq!(flow.step, LoginStep::ModelSelect);
        assert!(flow.available_models.is_empty());
        assert!(flow.selected_models.is_empty());
    }

    #[test]
    fn login_flow_toggle_model() {
        let mut flow = LoginFlowState::new()
            .with_provider("minimax".into())
            .with_key_and_defaults(
                "sk-test".into(),
                vec!["MiniMax-M3".into(), "MiniMax-M2.7".into()],
            );
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

    // -----------------------------------------------------------------------
    // Layer 1: with_fetched_models semantics
    // -----------------------------------------------------------------------

    #[test]
    fn fetched_models_replaces_list_and_selects_new() {
        let flow = LoginFlowState::new()
            .with_provider("minimax".into())
            .with_key_and_defaults("k".into(), vec!["A".into(), "B".into()]);
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
            .with_key_and_defaults("k".into(), vec!["A".into(), "B".into()]);
        flow.toggle_model("A");
        let flow = flow.with_fetched_models(vec!["A".into(), "B".into()]);
        assert!(!flow.selected_models.contains("A"));
        assert!(flow.selected_models.contains("B"));
    }

    #[test]
    fn fetched_models_drops_models_no_longer_returned() {
        let flow = LoginFlowState::new()
            .with_provider("minimax".into())
            .with_key_and_defaults("k".into(), vec!["A".into(), "B".into()]);
        let flow = flow.with_fetched_models(vec!["A".into()]);
        assert_eq!(flow.available_models, vec!["A"]);
        assert!(flow.selected_models.contains("A"));
        assert!(!flow.selected_models.contains("B"));
    }

    #[test]
    fn fetched_models_empty_list_clears_selection() {
        let flow = LoginFlowState::new()
            .with_provider("minimax".into())
            .with_key_and_defaults("k".into(), vec!["A".into()]);
        let flow = flow.with_fetched_models(vec![]);
        assert!(flow.available_models.is_empty());
        assert!(flow.selected_models.is_empty());
    }

    #[test]
    fn fetched_models_disjoint_list() {
        let flow = LoginFlowState::new()
            .with_provider("minimax".into())
            .with_key_and_defaults("k".into(), vec!["A".into()]);
        let flow = flow.with_fetched_models(vec!["X".into(), "Y".into()]);
        assert_eq!(flow.available_models, vec!["X", "Y"]);
        assert!(flow.selected_models.contains("X"));
        assert!(flow.selected_models.contains("Y"));
    }

    // -----------------------------------------------------------------------
    // Layer 2 helpers
    // -----------------------------------------------------------------------

    fn drive_to_model_select(provider: &str) -> AppState {
        let mut state = AppState::default();
        state.update(LoginFlowEvent::Start);
        state.update(LoginFlowEvent::SelectProvider {
            provider: provider.into(),
        });
        let defaults: Vec<String> = crate::provider_registry::find_provider(provider)
            .map(|m| {
                m.models
                    .iter()
                    .map(|model| model.name.to_string())
                    .collect()
            })
            .unwrap_or_default();
        state.update(LoginFlowEvent::SubmitKey {
            provider: provider.into(),
            key: "sk-test".into(),
        });
        let flow = state.login_flow.as_ref().unwrap();
        assert_eq!(flow.step, LoginStep::ModelSelect);
        assert_eq!(flow.available_models, defaults);
        assert!(flow.selected_models.len() == defaults.len());
        state
    }

    fn assert_flow_step(state: &AppState, step: LoginStep) {
        assert_eq!(state.login_flow.as_ref().unwrap().step, step);
    }

    fn assert_transient_contains(state: &AppState, needle: &str) {
        let content = state
            .transient_message
            .as_ref()
            .expect("transient expected");
        assert!(
            content.contains(needle),
            "transient should contain {needle}"
        );
    }

    // -----------------------------------------------------------------------
    // Layer 2: open / cancel
    // -----------------------------------------------------------------------

    #[test]
    fn key_input_esc_pops_to_provider_picker_not_close() {
        let mut state = AppState::default();
        state.update(LoginFlowEvent::Start);
        state.update(LoginFlowEvent::SelectProvider {
            provider: "minimax".into(),
        });
        match &state.open_dialog {
            Some(crate::commands::DialogState::PanelStack(s)) => {
                assert_eq!(s.len(), 2, "stack should be [provider, key_input]");
                assert_eq!(s.current().unwrap().id, "login-key");
            }
            other => panic!("expected PanelStack, got {other:?}"),
        }
        state.update(Event::dialog_back());
        match &state.open_dialog {
            Some(crate::commands::DialogState::PanelStack(s)) => {
                assert_eq!(s.len(), 1, "Esc must pop to root, not close");
                assert_eq!(s.current().unwrap().id, "login-provider");
            }
            other => panic!("Esc on key input must leave dialog open at root, got {other:?}"),
        }
    }

    #[test]
    fn login_command_opens_provider_picker() {
        let mut state = AppState::default();
        state.update(LoginFlowEvent::Start);
        assert!(state.open_dialog.is_some());
        assert!(state.login_flow.is_some());
        assert_flow_step(&state, LoginStep::ProviderPicker);
    }

    #[test]
    fn login_select_provider_pushes_key_input() {
        let mut state = AppState::default();
        state.update(LoginFlowEvent::Start);
        state.update(LoginFlowEvent::SelectProvider {
            provider: "minimax".into(),
        });
        assert_flow_step(&state, LoginStep::KeyInput);
        assert_eq!(state.login_flow.as_ref().unwrap().provider, "minimax");
    }

    #[test]
    fn login_submit_key_preserves_provider_when_empty() {
        let mut state = AppState::default();
        state.update(LoginFlowEvent::Start);
        state.update(LoginFlowEvent::SelectProvider {
            provider: "minimax".into(),
        });
        state.update(LoginFlowEvent::SubmitKey {
            provider: "".into(),
            key: "sk-test".into(),
        });
        let flow = state.login_flow.as_ref().unwrap();
        assert_eq!(flow.step, LoginStep::ModelSelect);
        assert_eq!(flow.provider, "minimax");
        assert_eq!(flow.key, "sk-test");
    }

    #[test]
    fn login_toggle_model_updates_selection() {
        let mut state = drive_to_model_select("minimax");
        let first = state.login_flow.as_ref().unwrap().available_models[0].clone();
        assert!(state
            .login_flow
            .as_ref()
            .unwrap()
            .selected_models
            .contains(&first));
        state.update(LoginFlowEvent::ToggleModel {
            model: first.clone(),
        });
        assert!(!state
            .login_flow
            .as_ref()
            .unwrap()
            .selected_models
            .contains(&first));
    }

    #[test]
    fn login_cancel_closes_dialog() {
        let mut state = AppState::default();
        state.update(LoginFlowEvent::Start);
        state.update(LoginFlowEvent::Cancel);
        assert!(state.open_dialog.is_none());
        assert!(state.login_flow.is_none());
    }

    // -----------------------------------------------------------------------
    // Layer 2: fetch scenarios
    // -----------------------------------------------------------------------

    #[test]
    fn s1_submit_key_immediately_shows_defaults() {
        let state = drive_to_model_select("minimax");
        let flow = state.login_flow.as_ref().unwrap();
        assert_eq!(flow.step, LoginStep::ModelSelect);
        assert!(!flow.available_models.is_empty());
    }

    #[test]
    fn s1_models_fetched_event_replaces_list() {
        let mut state = drive_to_model_select("minimax");
        state.update(LoginFlowEvent::ModelsFetched {
            provider: "minimax".into(),
            key: "sk-test".into(),
            models: vec!["new-A".into(), "new-B".into()],
        });
        let flow = state.login_flow.as_ref().unwrap();
        assert_eq!(flow.step, LoginStep::ModelSelect);
        assert_eq!(flow.available_models, vec!["new-A", "new-B"]);
        assert!(flow.selected_models.contains("new-A"));
        assert!(flow.selected_models.contains("new-B"));
    }

    #[test]
    fn s2_slow_fetch_user_can_toggle_before_it_returns() {
        let mut state = drive_to_model_select("minimax");
        let first_default = state.login_flow.as_ref().unwrap().available_models[0].clone();
        state.update(LoginFlowEvent::ToggleModel {
            model: first_default.clone(),
        });
        assert!(!state
            .login_flow
            .as_ref()
            .unwrap()
            .selected_models
            .contains(&first_default));

        let defaults = state.login_flow.as_ref().unwrap().available_models.clone();
        state.update(LoginFlowEvent::ModelsFetched {
            provider: "minimax".into(),
            key: "sk-test".into(),
            models: defaults.clone(),
        });
        let flow = state.login_flow.as_ref().unwrap();
        assert!(!flow.selected_models.contains(&first_default));
        for m in &defaults {
            if m != &first_default {
                assert!(flow.selected_models.contains(m));
            }
        }
    }

    #[test]
    fn s3_validation_failed_does_not_block_user() {
        let mut state = drive_to_model_select("minimax");
        state.update(LoginFlowEvent::ValidationFailed {
            provider: "minimax".into(),
            key: "sk-test".into(),
            error: "connection refused".into(),
        });
        assert_flow_step(&state, LoginStep::ModelSelect);
        assert_transient_contains(&state, "verify");
        assert!(!state
            .login_flow
            .as_ref()
            .unwrap()
            .available_models
            .is_empty());
    }

    #[test]
    fn s4_invalid_key_shows_transient_not_error_panel() {
        let mut state = drive_to_model_select("minimax");
        state.update(LoginFlowEvent::ValidationFailed {
            provider: "minimax".into(),
            key: "sk-test".into(),
            error: "API validation failed: 401 Unauthorized".into(),
        });
        assert_flow_step(&state, LoginStep::ModelSelect);
        assert!(state.transient_message.is_some());
    }

    #[test]
    fn s5_fetch_superset_preserves_toggle_and_selects_new() {
        let mut state = drive_to_model_select("minimax");
        let first = state.login_flow.as_ref().unwrap().available_models[0].clone();
        state.update(LoginFlowEvent::ToggleModel {
            model: first.clone(),
        });
        let mut new_list = state.login_flow.as_ref().unwrap().available_models.clone();
        new_list.push("brand-new-model".into());
        state.update(LoginFlowEvent::ModelsFetched {
            provider: "minimax".into(),
            key: "sk-test".into(),
            models: new_list,
        });
        let flow = state.login_flow.as_ref().unwrap();
        assert!(!flow.selected_models.contains(&first));
        assert!(flow.selected_models.contains("brand-new-model"));
    }

    #[test]
    fn s6_save_before_fetch_is_rejected() {
        let mut state = drive_to_model_select("minimax");
        state.update(LoginFlowEvent::Save);
        assert!(
            state.login_flow.is_some(),
            "save should be rejected before validation"
        );
        assert_transient_contains(&state, "validated");
    }

    #[test]
    fn s6b_late_fetch_after_rejected_save_is_ignored() {
        let mut state = drive_to_model_select("minimax");
        state.update(LoginFlowEvent::Save);
        assert!(state.login_flow.is_some());
        state.update(LoginFlowEvent::ModelsFetched {
            provider: "minimax".into(),
            key: "sk-test".into(),
            models: vec!["late".into()],
        });
        assert!(state.login_flow.is_some());
    }

    #[test]
    fn s7_cancel_before_fetch_then_fetch_is_ignored() {
        let mut state = drive_to_model_select("minimax");
        state.update(LoginFlowEvent::Cancel);
        assert!(state.login_flow.is_some(), "cancel should pop, not close");
        assert_flow_step(&state, LoginStep::ProviderPicker);
        state.update(LoginFlowEvent::ValidationFailed {
            provider: "minimax".into(),
            key: "sk-test".into(),
            error: "late".into(),
        });
        assert!(state.transient_message.is_none());
    }

    #[test]
    fn s8_empty_fetch_replaces_with_empty_list() {
        let mut state = drive_to_model_select("minimax");
        state.update(LoginFlowEvent::ModelsFetched {
            provider: "minimax".into(),
            key: "sk-test".into(),
            models: vec![],
        });
        let flow = state.login_flow.as_ref().unwrap();
        assert!(flow.available_models.is_empty());
        assert!(flow.selected_models.is_empty());
    }

    #[test]
    fn s9_unknown_provider_no_defaults() {
        let mut state = AppState::default();
        state.update(LoginFlowEvent::Start);
        state.update(LoginFlowEvent::SelectProvider {
            provider: "ghost".into(),
        });
        state.update(LoginFlowEvent::SubmitKey {
            provider: "ghost".into(),
            key: "k".into(),
        });
        let flow = state.login_flow.as_ref().unwrap();
        assert_eq!(flow.step, LoginStep::ModelSelect);
        assert!(flow.available_models.is_empty());
    }

    #[test]
    fn s10_fetch_subset_drops_missing_model() {
        let mut state = drive_to_model_select("minimax");
        let original = state.login_flow.as_ref().unwrap().available_models.clone();
        assert!(original.len() >= 2);
        let subset: Vec<String> = original.iter().take(1).cloned().collect();
        state.update(LoginFlowEvent::ModelsFetched {
            provider: "minimax".into(),
            key: "sk-test".into(),
            models: subset.clone(),
        });
        let flow = state.login_flow.as_ref().unwrap();
        assert_eq!(flow.available_models, subset);
        assert!(flow.selected_models.contains(&subset[0]));
    }

    #[test]
    fn s13_empty_key_still_shows_defaults() {
        let mut state = AppState::default();
        state.update(LoginFlowEvent::Start);
        state.update(LoginFlowEvent::SelectProvider {
            provider: "minimax".into(),
        });
        state.update(LoginFlowEvent::SubmitKey {
            provider: "minimax".into(),
            key: "".into(),
        });
        let flow = state.login_flow.as_ref().unwrap();
        assert_eq!(flow.step, LoginStep::ModelSelect);
        assert!(!flow.available_models.is_empty());
    }
}
