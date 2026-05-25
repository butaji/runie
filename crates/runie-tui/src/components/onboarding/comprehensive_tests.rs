//! Comprehensive tests for the onboarding component.

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use crate::tui::state::{AppState, TuiMode, Msg, Cmd};
    use crate::tui::update::onboarding::handle_onboarding_msg;
    use crate::components::onboarding::{Onboarding, OnboardingStep};
    use runie_ai::model_fetcher::ModelInfo;

    const OPENAI_INDEX: usize = 15;

    fn setup() -> AppState {
        let mut state = AppState::default();
        state.onboarding = Some(Onboarding::new());
        state.mode = TuiMode::Onboarding;
        state
    }

    // ─── Category 1: Step Transitions ───────────────────────────────────────────

    #[test]
    fn test_welcome_to_provider_select() {
        let mut state = setup();
        assert_eq!(state.onboarding.as_ref().unwrap().step, OnboardingStep::Welcome);
        let cmds = handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        assert!(cmds.is_empty());
        assert_eq!(state.onboarding.as_ref().unwrap().step, OnboardingStep::ProviderSelect);
    }

    #[test]
    fn test_provider_select_to_key_input_with_selection() {
        let mut state = setup();
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().update_search("");
        state.onboarding.as_mut().unwrap().select_provider(OPENAI_INDEX);
        let cmds = handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        assert!(cmds.is_empty());
        assert_eq!(state.onboarding.as_ref().unwrap().step, OnboardingStep::KeyInput);
    }

    #[test]
    fn test_model_select_to_complete_with_selection() {
        let mut state = setup();
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().update_search("");
        state.onboarding.as_mut().unwrap().select_provider(OPENAI_INDEX);
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().api_key_input = "sk-test".to_string();
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().is_fetching_models = false;
        state.onboarding.as_mut().unwrap().step = OnboardingStep::ModelSelect;
        state.onboarding.as_mut().unwrap().enter_step();
        state.onboarding.as_mut().unwrap().select_model(0);
        let cmds = handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        assert!(cmds.is_empty());
        assert_eq!(state.onboarding.as_ref().unwrap().step, OnboardingStep::Complete);
    }

    // ─── Category 2: Navigation ───────────────────────────────────────────────

    #[test]
    fn test_navigate_providers() {
        let mut state = setup();
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().update_search("");
        assert_eq!(state.onboarding.as_ref().unwrap().selected_item, 0);
        handle_onboarding_msg(&mut state, Msg::OnboardingNavigateDown);
        assert_eq!(state.onboarding.as_ref().unwrap().selected_item, 1);
        handle_onboarding_msg(&mut state, Msg::OnboardingNavigateUp);
        assert_eq!(state.onboarding.as_ref().unwrap().selected_item, 0);
        handle_onboarding_msg(&mut state, Msg::OnboardingNavigateUp);
        assert_eq!(state.onboarding.as_ref().unwrap().selected_item, 0);
    }

    #[test]
    fn test_navigate_models() {
        let mut state = setup();
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().update_search("");
        state.onboarding.as_mut().unwrap().select_provider(OPENAI_INDEX);
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().api_key_input = "sk-test".to_string();
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().is_fetching_models = false;
        state.onboarding.as_mut().unwrap().step = OnboardingStep::ModelSelect;
        state.onboarding.as_mut().unwrap().enter_step();
        let count = state.onboarding.as_ref().unwrap().models.len();
        assert!(count > 1);
        for _ in 0..count { handle_onboarding_msg(&mut state, Msg::OnboardingNavigateDown); }
        assert_eq!(state.onboarding.as_ref().unwrap().selected_item, count - 1);
        handle_onboarding_msg(&mut state, Msg::OnboardingNavigateUp);
        assert_eq!(state.onboarding.as_ref().unwrap().selected_item, count - 2);
    }

    #[test]
    fn test_navigate_complete_yes_no() {
        let mut state = setup();
        let o = state.onboarding.as_mut().unwrap();
        o.step = OnboardingStep::Complete;
        o.selected_item = 0;
        drop(o);
        assert_eq!(state.onboarding.as_ref().unwrap().selected_item, 0);
        handle_onboarding_msg(&mut state, Msg::OnboardingNavigateDown);
        assert_eq!(state.onboarding.as_ref().unwrap().selected_item, 1);
        handle_onboarding_msg(&mut state, Msg::OnboardingNavigateDown);
        assert_eq!(state.onboarding.as_ref().unwrap().selected_item, 1);
        handle_onboarding_msg(&mut state, Msg::OnboardingNavigateUp);
        assert_eq!(state.onboarding.as_ref().unwrap().selected_item, 0);
    }

    #[test]
    fn test_back_navigation() {
        let mut state = setup();
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        assert_eq!(state.onboarding.as_ref().unwrap().step, OnboardingStep::ProviderSelect);
        handle_onboarding_msg(&mut state, Msg::OnboardingBack);
        assert_eq!(state.onboarding.as_ref().unwrap().step, OnboardingStep::Welcome);
        handle_onboarding_msg(&mut state, Msg::OnboardingBack);
        assert_eq!(state.onboarding.as_ref().unwrap().step, OnboardingStep::Welcome);
    }

    #[test]
    fn test_back_navigation_full_flow() {
        let mut state = setup();
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().update_search("");
        state.onboarding.as_mut().unwrap().select_provider(OPENAI_INDEX);
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().api_key_input = "sk-test".to_string();
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().is_fetching_models = false;
        state.onboarding.as_mut().unwrap().step = OnboardingStep::ModelSelect;
        state.onboarding.as_mut().unwrap().enter_step();
        state.onboarding.as_mut().unwrap().select_model(0);
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        assert_eq!(state.onboarding.as_ref().unwrap().step, OnboardingStep::Complete);
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        assert_eq!(state.onboarding.as_ref().unwrap().step, OnboardingStep::ProviderSelect);
        handle_onboarding_msg(&mut state, Msg::OnboardingBack);
        assert_eq!(state.onboarding.as_ref().unwrap().step, OnboardingStep::Welcome);
    }

    // ─── Category 3: Provider Selection ───────────────────────────────────────

    #[test]
    fn test_select_provider_by_index() {
        let mut state = setup();
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().update_search("");
        handle_onboarding_msg(&mut state, Msg::OnboardingSelectProvider(0));
        assert!(state.onboarding.as_ref().unwrap().selected_provider.is_some());
    }

    #[test]
    fn test_provider_filtering() {
        let mut state = setup();
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().update_search("");
        handle_onboarding_msg(&mut state, Msg::OnboardingSearchInput('o'));
        handle_onboarding_msg(&mut state, Msg::OnboardingSearchInput('p'));
        handle_onboarding_msg(&mut state, Msg::OnboardingSearchInput('e'));
        assert_eq!(state.onboarding.as_ref().unwrap().get_filtered_provider_count(), 2);
    }

    #[test]
    fn test_empty_filter_no_selection() {
        let mut state = setup();
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().update_search("");
        state.onboarding.as_mut().unwrap().update_search("ope");
        assert_eq!(state.onboarding.as_ref().unwrap().get_filtered_provider_count(), 2);
        state.onboarding.as_mut().unwrap().clear_search();
        assert!(state.onboarding.as_ref().unwrap().search_query.is_empty());
        assert_eq!(state.onboarding.as_ref().unwrap().get_filtered_provider_count(),
            state.onboarding.as_ref().unwrap().providers.len());
    }

    // ─── Category 4: API Key ─────────────────────────────────────────────────

    #[test]
    fn test_invalid_key_stays() {
        let mut state = setup();
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().update_search("");
        state.onboarding.as_mut().unwrap().select_provider(OPENAI_INDEX);
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().api_key_input = "pk-wrong".to_string();
        let cmds = handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        assert!(cmds.is_empty());
        assert_eq!(state.onboarding.as_ref().unwrap().step, OnboardingStep::KeyInput);
        assert!(state.onboarding.as_ref().unwrap().error_message.is_some());
    }

    #[test]
    fn test_key_backspace() {
        let mut state = setup();
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().update_search("");
        state.onboarding.as_mut().unwrap().select_provider(OPENAI_INDEX);
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        for c in "sk-test".chars() {
            handle_onboarding_msg(&mut state, Msg::OnboardingKeyInput(c));
        }
        assert_eq!(state.onboarding.as_ref().unwrap().api_key_input, "sk-test");
        handle_onboarding_msg(&mut state, Msg::OnboardingKeyBackspace);
        assert_eq!(state.onboarding.as_ref().unwrap().api_key_input, "sk-tes");
        handle_onboarding_msg(&mut state, Msg::OnboardingKeyBackspace);
        assert_eq!(state.onboarding.as_ref().unwrap().api_key_input, "sk-te");
        for _ in 0..6 { handle_onboarding_msg(&mut state, Msg::OnboardingKeyBackspace); }
        assert_eq!(state.onboarding.as_ref().unwrap().api_key_input, "");
    }

    #[test]
    fn test_empty_key_rejected() {
        let mut state = setup();
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().update_search("");
        state.onboarding.as_mut().unwrap().select_provider(OPENAI_INDEX);
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().api_key_input = "".to_string();
        let cmds = handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        assert!(cmds.is_empty());
        assert!(state.onboarding.as_ref().unwrap().error_message.is_some());
    }

    #[test]
    fn test_key_input_append() {
        let mut state = setup();
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().update_search("");
        state.onboarding.as_mut().unwrap().select_provider(OPENAI_INDEX);
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        for c in "abc".chars() {
            handle_onboarding_msg(&mut state, Msg::OnboardingKeyInput(c));
        }
        assert_eq!(state.onboarding.as_ref().unwrap().api_key_input, "abc");
    }

    // ─── Category 5: Model Fetching ──────────────────────────────────────────

    #[test]
    fn test_fetch_failure_uses_fallback() {
        let mut state = setup();
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().update_search("");
        state.onboarding.as_mut().unwrap().select_provider(OPENAI_INDEX);
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().api_key_input = "sk-test".to_string();
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().is_fetching_models = false;
        let cmds = handle_onboarding_msg(&mut state, Msg::ModelsFetchFailed("Network error".to_string()));
        let o = state.onboarding.as_ref().unwrap();
        assert!(!o.is_fetching_models);
        assert!(o.error_message.as_ref().unwrap().contains("Network error"));
        assert_eq!(o.models.len(), 3);
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_fetched_models_sorted() {
        let mut state = setup();
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().update_search("");
        state.onboarding.as_mut().unwrap().select_provider(OPENAI_INDEX);
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().api_key_input = "sk-test".to_string();
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().is_fetching_models = false;
        let fetched = vec![
            ModelInfo { name: "Zebra".to_string(), id: "z".to_string() },
            ModelInfo { name: "Alpha".to_string(), id: "a".to_string() },
            ModelInfo { name: "Mango".to_string(), id: "m".to_string() },
        ];
        handle_onboarding_msg(&mut state, Msg::ModelsFetched(fetched));
        let o = state.onboarding.as_ref().unwrap();
        assert_eq!(o.models[0].name, "Alpha");
        assert_eq!(o.models[1].name, "Mango");
        assert_eq!(o.models[2].name, "Zebra");
    }

    // ─── Category 6: Complete Step ───────────────────────────────────────────

    #[test]
    fn test_complete_yes_restarts() {
        let mut state = setup();
        let o = state.onboarding.as_mut().unwrap();
        o.step = OnboardingStep::Complete;
        o.selected_item = 0;
        o.selected_provider = Some(OPENAI_INDEX);
        o.selected_model = Some(0);
        o.api_key_input = "sk-test".to_string();
        o.models.push(crate::components::onboarding::ModelOption {
            name: "GPT-4o".to_string(), id: "gpt-4o".to_string(), description: "".to_string()
        });
        drop(o);
        let cmds = handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        let o = state.onboarding.as_ref().unwrap();
        assert_eq!(o.step, OnboardingStep::ProviderSelect);
        assert!(o.selected_provider.is_none());
        assert!(o.api_key_input.is_empty());
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_complete_no_exits() {
        let mut state = setup();
        let o = state.onboarding.as_mut().unwrap();
        o.step = OnboardingStep::Complete;
        o.selected_item = 1;
        o.selected_provider = Some(OPENAI_INDEX);
        o.selected_model = Some(0);
        o.api_key_input = "sk-test".to_string();
        o.models.push(crate::components::onboarding::ModelOption {
            name: "GPT-4o".to_string(), id: "gpt-4o".to_string(), description: "".to_string()
        });
        drop(o);
        let cmds = handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        assert!(state.onboarding.is_none());
        assert_eq!(state.mode, TuiMode::Chat);
        assert_eq!(cmds.len(), 1);
        assert!(matches!(&cmds[0], Cmd::SaveSettings { provider, model, api_key }
            if provider == "openai" && model == "gpt-4o" && api_key == "sk-test"));
    }

    #[test]
    fn test_complete_settings_saved() {
        let mut state = setup();
        let o = state.onboarding.as_mut().unwrap();
        o.step = OnboardingStep::Complete;
        o.selected_item = 1;
        o.selected_provider = Some(OPENAI_INDEX);
        o.selected_model = Some(0);
        o.api_key_input = "sk-secret".to_string();
        o.models.push(crate::components::onboarding::ModelOption {
            name: "GPT-4o".to_string(), id: "gpt-4o".to_string(), description: "".to_string()
        });
        drop(o);
        let cmds = handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        assert!(state.onboarding.is_none());
        assert_eq!(state.mode, TuiMode::Chat);
        match &cmds[0] {
            Cmd::SaveSettings { provider, api_key, .. } => {
                assert_eq!(provider, "openai");
                assert_eq!(api_key, "sk-secret");
            }
            _ => panic!("Expected SaveSettings"),
        }
    }

    #[test]
    fn test_complete_yes_clears_state() {
        let mut state = setup();
        let o = state.onboarding.as_mut().unwrap();
        o.step = OnboardingStep::Complete;
        o.selected_item = 0;
        o.selected_provider = Some(5);
        o.selected_model = Some(3);
        o.api_key_input = "sk-old".to_string();
        o.search_query = "test".to_string();
        o.error_message = Some("old error".to_string());
        o.models.push(crate::components::onboarding::ModelOption {
            name: "Test".to_string(), id: "test".to_string(), description: "".to_string()
        });
        drop(o);
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        let o = state.onboarding.as_ref().unwrap();
        assert!(o.selected_provider.is_none());
        assert!(o.selected_model.is_none());
        assert!(o.api_key_input.is_empty());
        assert!(o.search_query.is_empty());
        assert!(o.error_message.is_none());
    }
}
