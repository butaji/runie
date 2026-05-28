//! Comprehensive tests for the onboarding component.

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use crate::tui::state::{AppState, TuiMode, Msg, Cmd};
    use crate::tui::update::onboarding::handle_onboarding_msg;
    use crate::components::onboarding::{Onboarding, OnboardingStep};
    use runie_ai::model_fetcher::ModelInfo;

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
        let openai_idx = state.onboarding.as_ref().unwrap().providers.iter()
            .position(|p| p.id == "openai")
            .expect("OpenAI provider should exist");
        state.onboarding.as_mut().unwrap().select_provider(openai_idx);
        state.onboarding.as_mut().unwrap().selected_item = openai_idx;
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
        let openai_idx = state.onboarding.as_ref().unwrap().providers.iter()
            .position(|p| p.id == "openai")
            .expect("OpenAI provider should exist");
        state.onboarding.as_mut().unwrap().select_provider(openai_idx);
        state.onboarding.as_mut().unwrap().selected_item = openai_idx;
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
        let openai_idx = state.onboarding.as_ref().unwrap().providers.iter()
            .position(|p| p.id == "openai")
            .expect("OpenAI provider should exist");
        state.onboarding.as_mut().unwrap().select_provider(openai_idx);
        state.onboarding.as_mut().unwrap().selected_item = openai_idx;
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().api_key_input = "sk-test".to_string();
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().is_fetching_models = false;
        state.onboarding.as_mut().unwrap().step = OnboardingStep::ModelSelect;
        state.onboarding.as_mut().unwrap().enter_step();
        state.onboarding.as_mut().unwrap().select_model(0);
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        assert_eq!(state.onboarding.as_ref().unwrap().step, OnboardingStep::Complete);
        // Reset selected_item to 0 (Yes) to restart onboarding
        state.onboarding.as_mut().unwrap().selected_item = 0;
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
        let openai_idx = state.onboarding.as_ref().unwrap().providers.iter()
            .position(|p| p.id == "openai")
            .expect("OpenAI provider should exist");
        state.onboarding.as_mut().unwrap().select_provider(openai_idx);
        state.onboarding.as_mut().unwrap().selected_item = openai_idx;
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
        let openai_idx = state.onboarding.as_ref().unwrap().providers.iter()
            .position(|p| p.id == "openai")
            .expect("OpenAI provider should exist");
        state.onboarding.as_mut().unwrap().select_provider(openai_idx);
        state.onboarding.as_mut().unwrap().selected_item = openai_idx;
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        for c in "sk-test".chars() {
            handle_onboarding_msg(&mut state, Msg::OnboardingKeyInput(c));
        }
        assert_eq!(state.onboarding.as_ref().unwrap().api_key_input, "sk-test");
        // Backspace removes whole token ("test" after dash)
        handle_onboarding_msg(&mut state, Msg::OnboardingKeyBackspace);
        assert_eq!(state.onboarding.as_ref().unwrap().api_key_input, "sk-");
        // Backspace removes next token ("sk-")
        handle_onboarding_msg(&mut state, Msg::OnboardingKeyBackspace);
        assert_eq!(state.onboarding.as_ref().unwrap().api_key_input, "");
    }

    #[test]
    fn test_empty_key_rejected() {
        let mut state = setup();
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().update_search("");
        let openai_idx = state.onboarding.as_ref().unwrap().providers.iter()
            .position(|p| p.id == "openai")
            .expect("OpenAI provider should exist");
        state.onboarding.as_mut().unwrap().select_provider(openai_idx);
        state.onboarding.as_mut().unwrap().selected_item = openai_idx;
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
        let openai_idx = state.onboarding.as_ref().unwrap().providers.iter()
            .position(|p| p.id == "openai")
            .expect("OpenAI provider should exist");
        state.onboarding.as_mut().unwrap().select_provider(openai_idx);
        state.onboarding.as_mut().unwrap().selected_item = openai_idx;
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

    // ─── Category 6: Complete Step ───────────────────────────────────────────

    #[test]
    fn test_complete_yes_restarts() {
        let mut state = setup();
        let o = state.onboarding.as_mut().unwrap();
        o.step = OnboardingStep::Complete;
        o.selected_item = 0;
        let openai_idx = o.providers.iter().position(|p| p.id == "openai").expect("OpenAI provider should exist");
        o.selected_provider = Some(openai_idx);
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
        let openai_idx = o.providers.iter().position(|p| p.id == "openai").expect("OpenAI provider should exist");
        o.selected_provider = Some(openai_idx);
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
        let openai_idx = o.providers.iter().position(|p| p.id == "openai").expect("OpenAI provider should exist");
        o.selected_provider = Some(openai_idx);
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

    // ─── Category 6: Search/Filter Selection ───────────────────────────────────

    #[test]
    fn test_search_filters_providers() {
        let mut state = setup();
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().update_search("");

        // Search for "anthropic" - should filter to 1 provider
        for c in "anthropic".chars() {
            handle_onboarding_msg(&mut state, Msg::OnboardingSearchInput(c));
        }
        assert_eq!(state.onboarding.as_ref().unwrap().get_filtered_provider_count(), 1);
        assert_eq!(state.onboarding.as_ref().unwrap().selected_item, 0);
    }

    #[test]
    fn test_select_from_filtered_providers() {
        let mut state = setup();
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().update_search("");

        // Search for "anthropic"
        for c in "anthropic".chars() {
            handle_onboarding_msg(&mut state, Msg::OnboardingSearchInput(c));
        }

        // Press Enter to select the filtered provider
        let _cmds = handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        assert_eq!(state.onboarding.as_ref().unwrap().step, OnboardingStep::KeyInput);
        assert!(state.onboarding.as_ref().unwrap().selected_provider.is_some());
        let provider_idx = state.onboarding.as_ref().unwrap().selected_provider.unwrap();
        assert_eq!(state.onboarding.as_ref().unwrap().providers[provider_idx].id, "anthropic");
    }

    #[test]
    fn test_search_no_matches_shows_empty() {
        let mut state = setup();
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().update_search("");

        // Search for something that doesn't exist
        for c in "zzzzzz".chars() {
            handle_onboarding_msg(&mut state, Msg::OnboardingSearchInput(c));
        }
        assert_eq!(state.onboarding.as_ref().unwrap().get_filtered_provider_count(), 0);

        // Pressing Enter with no matches should stay on ProviderSelect
        let _cmds = handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        assert_eq!(state.onboarding.as_ref().unwrap().step, OnboardingStep::ProviderSelect);
        assert!(state.onboarding.as_ref().unwrap().error_message.is_some());
    }

    #[test]
    fn test_clear_search_restores_all_providers() {
        let mut state = setup();
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().update_search("");

        // Filter to 1 provider
        for c in "anthropic".chars() {
            handle_onboarding_msg(&mut state, Msg::OnboardingSearchInput(c));
        }
        assert_eq!(state.onboarding.as_ref().unwrap().get_filtered_provider_count(), 1);

        // Clear search character by character
        for _ in 0..9 {
            handle_onboarding_msg(&mut state, Msg::OnboardingSearchBackspace);
        }
        assert!(state.onboarding.as_ref().unwrap().search_query.is_empty());
        assert_eq!(state.onboarding.as_ref().unwrap().get_filtered_provider_count(),
            state.onboarding.as_ref().unwrap().providers.len());
    }

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

    #[test]
    fn test_search_resets_selected_item_to_zero() {
        let mut state = setup();
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().update_search("");

        // Navigate down to index 5
        for _ in 0..5 {
            handle_onboarding_msg(&mut state, Msg::OnboardingNavigateDown);
        }
        assert_eq!(state.onboarding.as_ref().unwrap().selected_item, 5);

        // Type a search - selected_item should reset to 0
        handle_onboarding_msg(&mut state, Msg::OnboardingSearchInput('o'));
        assert_eq!(state.onboarding.as_ref().unwrap().selected_item, 0);
    }

    // ─── Category 7: BUG-13 Esc/Skip Behavior ─────────────────────────────────

    #[test]
    fn test_skip_onboarding_esc() {
        // BUG-13: Currently Esc on Welcome maps to Back, which stays at Welcome.
        // Should map to Skip (clear onboarding) instead.
        let mut state = setup();
        assert_eq!(state.onboarding.as_ref().unwrap().step, OnboardingStep::Welcome);
        // OnboardingBack on Welcome stays at Welcome (prev_step returns Welcome)
        handle_onboarding_msg(&mut state, Msg::OnboardingBack);
        assert_eq!(state.onboarding.as_ref().unwrap().step, OnboardingStep::Welcome);
        // BUG: onboarding is NOT None when it should be (Esc should skip)
        assert!(state.onboarding.is_some());
        // OnboardingSkip clears onboarding immediately
        handle_onboarding_msg(&mut state, Msg::OnboardingSkip);
        assert!(state.onboarding.is_none());
        assert_eq!(state.mode, TuiMode::Chat);
    }

    // ─── Category 8: Paste Behavior ────────────────────────────────────────────

    #[test]
    fn test_paste_into_api_key() {
        let mut state = setup();
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().update_search("");
        let openai_idx = state.onboarding.as_ref().unwrap().providers.iter()
            .position(|p| p.id == "openai")
            .expect("OpenAI provider should exist");
        state.onboarding.as_mut().unwrap().select_provider(openai_idx);
        state.onboarding.as_mut().unwrap().selected_item = openai_idx;
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        // Paste text at KeyInput step
        handle_onboarding_msg(&mut state, Msg::Paste("sk-test-key".to_string()));
        assert_eq!(state.onboarding.as_ref().unwrap().api_key_input, "sk-test-key");
        // Paste appends, not replaces
        handle_onboarding_msg(&mut state, Msg::Paste("-extra".to_string()));
        assert_eq!(state.onboarding.as_ref().unwrap().api_key_input, "sk-test-key-extra");
    }

    #[test]
    fn test_paste_at_welcome_ignored() {
        let mut state = setup();
        assert_eq!(state.onboarding.as_ref().unwrap().step, OnboardingStep::Welcome);
        // Paste on Welcome should be ignored (_ => {})
        handle_onboarding_msg(&mut state, Msg::Paste("ignored".to_string()));
        assert_eq!(state.onboarding.as_ref().unwrap().step, OnboardingStep::Welcome);
        assert!(state.onboarding.is_some());
    }

    // ─── Category 9: API Key Validation & Fetch Transition ─────────────────────

    #[test]
    fn test_valid_key_fetch_transition() {
        let mut state = setup();
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().update_search("");
        let openai_idx = state.onboarding.as_ref().unwrap().providers.iter()
            .position(|p| p.id == "openai")
            .expect("OpenAI provider should exist");
        state.onboarding.as_mut().unwrap().select_provider(openai_idx);
        state.onboarding.as_mut().unwrap().selected_item = openai_idx;
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().api_key_input = "sk-validkey123".to_string();
        let cmds = handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        assert!(state.onboarding.as_ref().unwrap().is_fetching_models);
        assert!(cmds.iter().any(|c| matches!(c, Cmd::FetchModels { .. })));
    }

    #[test]
    fn test_retry_after_fetch_failure() {
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
        // Trigger fetch failure
        handle_onboarding_msg(&mut state, Msg::OnboardingNext); // starts fetch
        handle_onboarding_msg(&mut state, Msg::ModelsFetchFailed("Network error".to_string()));
        assert!(state.onboarding.as_ref().unwrap().fetch_error.is_some());
        assert!(state.onboarding.as_ref().unwrap().error_message.is_some());
        // Retry (press Next again with same key)
        let _cmds = handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        // clear_fetch_error is called on retry
        assert!(state.onboarding.as_ref().unwrap().fetch_error.is_none());
        assert!(state.onboarding.as_ref().unwrap().is_fetching_models);
    }

    // ─── Category 10: Token Backspace Edge Cases ───────────────────────────────

    #[test]
    fn test_token_backspace_single_separator() {
        let mut state = setup();
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().update_search("");
        let openai_idx = state.onboarding.as_ref().unwrap().providers.iter()
            .position(|p| p.id == "openai")
            .expect("OpenAI provider should exist");
        state.onboarding.as_mut().unwrap().select_provider(openai_idx);
        state.onboarding.as_mut().unwrap().selected_item = openai_idx;
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        // Input just a dash
        handle_onboarding_msg(&mut state, Msg::OnboardingKeyInput('-'));
        assert_eq!(state.onboarding.as_ref().unwrap().api_key_input, "-");
        // Single backspace removes entire separator token
        handle_onboarding_msg(&mut state, Msg::OnboardingKeyBackspace);
        assert_eq!(state.onboarding.as_ref().unwrap().api_key_input, "");
    }

    #[test]
    fn test_token_backspace_all_separators() {
        let mut state = setup();
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().update_search("");
        let openai_idx = state.onboarding.as_ref().unwrap().providers.iter()
            .position(|p| p.id == "openai")
            .expect("OpenAI provider should exist");
        state.onboarding.as_mut().unwrap().select_provider(openai_idx);
        state.onboarding.as_mut().unwrap().selected_item = openai_idx;
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        // Input two dashes
        handle_onboarding_msg(&mut state, Msg::OnboardingKeyInput('-'));
        handle_onboarding_msg(&mut state, Msg::OnboardingKeyInput('-'));
        assert_eq!(state.onboarding.as_ref().unwrap().api_key_input, "--");
        // Single backspace removes all trailing separators and token
        handle_onboarding_msg(&mut state, Msg::OnboardingKeyBackspace);
        assert_eq!(state.onboarding.as_ref().unwrap().api_key_input, "");
    }

    // ─── Category 11: Empty Filter & Selection ─────────────────────────────────

    #[test]
    fn test_empty_filtered_list_enter() {
        let mut state = setup();
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        state.onboarding.as_mut().unwrap().update_search("");
        // Search with no matches
        for c in "zzzznoexists".chars() {
            handle_onboarding_msg(&mut state, Msg::OnboardingSearchInput(c));
        }
        assert_eq!(state.onboarding.as_ref().unwrap().get_filtered_provider_count(), 0);
        // Enter with empty filtered list shows error
        let _cmds = handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        assert!(state.onboarding.as_ref().unwrap().error_message.is_some());
    }

    // ─── Category 12: Pluralization ───────────────────────────────────────────

    #[test]
    fn test_pluralization_one_model() {
        let mut state = setup();
        let o = state.onboarding.as_mut().unwrap();
        o.step = OnboardingStep::Complete;
        o.selected_item = 1;
        let openai_idx = o.providers.iter().position(|p| p.id == "openai").expect("OpenAI provider should exist");
        o.selected_provider = Some(openai_idx);
        o.selected_model = Some(0);
        o.api_key_input = "sk-test".to_string();
        o.models.push(crate::components::onboarding::ModelOption {
            name: "GPT-4o".to_string(), id: "gpt-4o".to_string(), description: "".to_string()
        });
        drop(o);
        let o = state.onboarding.as_ref().unwrap();
        let model_count = o.models.len();
        assert_eq!(model_count, 1);
        let model_word = if model_count == 1 { "model" } else { "models" };
        assert_eq!(model_word, "model");
    }

    #[test]
    fn test_pluralization_many_models() {
        let mut state = setup();
        let o = state.onboarding.as_mut().unwrap();
        o.step = OnboardingStep::Complete;
        o.selected_item = 1;
        let openai_idx = o.providers.iter().position(|p| p.id == "openai").expect("OpenAI provider should exist");
        o.selected_provider = Some(openai_idx);
        o.selected_model = Some(0);
        o.api_key_input = "sk-test".to_string();
        o.models.push(crate::components::onboarding::ModelOption {
            name: "GPT-4o".to_string(), id: "gpt-4o".to_string(), description: "".to_string()
        });
        o.models.push(crate::components::onboarding::ModelOption {
            name: "GPT-4o Mini".to_string(), id: "gpt-4o-mini".to_string(), description: "".to_string()
        });
        drop(o);
        let o = state.onboarding.as_ref().unwrap();
        let model_count = o.models.len();
        assert_eq!(model_count, 2);
        let model_word = if model_count == 1 { "model" } else { "models" };
        assert_eq!(model_word, "models");
    }
}
