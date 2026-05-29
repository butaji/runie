//! Provider selection tests

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use crate::tui::state::{AppState, TuiMode, Msg, Cmd};
    use crate::tui::update::onboarding::handle_onboarding_msg;
    use crate::components::onboarding::{Onboarding, OnboardingStep};

    fn setup() -> AppState {
        let mut state = AppState::default();
        state.onboarding = Some(Onboarding::new(false));
        state.mode = TuiMode::Onboarding;
        state
    }

    // ─── Provider Selection ────────────────────────────────────────────────────

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

    // ─── Search/Filter Selection ───────────────────────────────────────────────

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
}
