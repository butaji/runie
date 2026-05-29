//! Model selection and fetching tests

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use crate::tui::state::{AppState, TuiMode, Msg, Cmd};
    use crate::tui::update::onboarding::handle_onboarding_msg;
    use crate::components::onboarding::{Onboarding, OnboardingStep};
    use runie_ai::model_fetcher::ModelInfo;

    fn setup() -> AppState {
        let mut state = AppState::default();
        state.onboarding = Some(Onboarding::new(false));
        state.mode = TuiMode::Onboarding;
        state
    }

    // ─── Model Fetching ───────────────────────────────────────────────────────

    #[test]
    fn test_fetch_failure_uses_fallback() {
        let mut state = setup();
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().update_search("");
        let openai_idx = state.onboarding.as_ref().unwrap().providers.iter()
            .position(|p| p.id == "openai")
            .expect("OpenAI provider should exist");
        state.onboarding.as_mut().unwrap().select_provider(openai_idx);
        state.onboarding.as_mut().unwrap().selected_item = openai_idx;
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
        let openai_idx = state.onboarding.as_ref().unwrap().providers.iter()
            .position(|p| p.id == "openai")
            .expect("OpenAI provider should exist");
        state.onboarding.as_mut().unwrap().select_provider(openai_idx);
        state.onboarding.as_mut().unwrap().selected_item = openai_idx;
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

    // ─── Search/Filter Models ────────────────────────────────────────────────

    #[test]
    fn test_search_filters_models() {
        let mut state = setup();
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().update_search("");
        let openai_idx = state.onboarding.as_ref().unwrap().providers.iter()
            .position(|p| p.id == "openai")
            .expect("OpenAI provider should exist");
        state.onboarding.as_mut().unwrap().select_provider(openai_idx);
        state.onboarding.as_mut().unwrap().selected_item = openai_idx;
        state.onboarding.as_mut().unwrap().selected_item = openai_idx;
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().api_key_input = "sk-test".to_string();
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        // Manually transition to ModelSelect (models were populated by select_provider)
        state.onboarding.as_mut().unwrap().is_fetching_models = false;
        state.onboarding.as_mut().unwrap().step = OnboardingStep::ModelSelect;
        state.onboarding.as_mut().unwrap().enter_step();

        // Now on ModelSelect, search for "gpt-4"
        for c in "gpt-4".chars() {
            handle_onboarding_msg(&mut state, Msg::OnboardingSearchInput(c));
        }
        let filtered_count = state.onboarding.as_ref().unwrap().get_filtered_model_count();
        assert!(filtered_count > 0, "Should find some gpt-4 models");
        assert!(filtered_count < state.onboarding.as_ref().unwrap().models.len(),
            "Should filter down from all models");
    }

    #[test]
    fn test_select_from_filtered_models() {
        let mut state = setup();
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().update_search("");
        let openai_idx = state.onboarding.as_ref().unwrap().providers.iter()
            .position(|p| p.id == "openai")
            .expect("OpenAI provider should exist");
        state.onboarding.as_mut().unwrap().select_provider(openai_idx);
        state.onboarding.as_mut().unwrap().selected_item = openai_idx;
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().api_key_input = "sk-test".to_string();
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        // Manually transition to ModelSelect (models were populated by select_provider)
        state.onboarding.as_mut().unwrap().is_fetching_models = false;
        state.onboarding.as_mut().unwrap().step = OnboardingStep::ModelSelect;
        state.onboarding.as_mut().unwrap().enter_step();

        // Search for "gpt-4o"
        for c in "gpt-4o".chars() {
            handle_onboarding_msg(&mut state, Msg::OnboardingSearchInput(c));
        }

        // Press Enter to select the filtered model
        let _cmds = handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        assert_eq!(state.onboarding.as_ref().unwrap().step, OnboardingStep::Complete);
        assert!(state.onboarding.as_ref().unwrap().selected_model.is_some());
    }
}
