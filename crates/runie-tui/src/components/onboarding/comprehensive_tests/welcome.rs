//! Welcome step tests - transitions, navigation, escape/skip behavior

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

    // ─── Step Transitions ───────────────────────────────────────────────────────

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

    // ─── Navigation ────────────────────────────────────────────────────────────

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
        let _ = o;
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

    // ─── BUG-13 Esc/Skip Behavior ──────────────────────────────────────────────

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

    // ─── Paste Behavior ────────────────────────────────────────────────────────

    #[test]
    fn test_paste_at_welcome_ignored() {
        let mut state = setup();
        assert_eq!(state.onboarding.as_ref().unwrap().step, OnboardingStep::Welcome);
        // Paste on Welcome should be ignored (_ => {})
        handle_onboarding_msg(&mut state, Msg::Paste("ignored".to_string()));
        assert_eq!(state.onboarding.as_ref().unwrap().step, OnboardingStep::Welcome);
        assert!(state.onboarding.is_some());
    }

    // ─── Empty Filter & Selection ─────────────────────────────────────────────

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
}
