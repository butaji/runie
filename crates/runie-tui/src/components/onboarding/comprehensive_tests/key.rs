//! API key input and validation tests

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

    // ─── API Key Input ────────────────────────────────────────────────────────

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

    // ─── Paste Behavior ───────────────────────────────────────────────────────

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

    // ─── API Key Validation & Fetch Transition ───────────────────────────────

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

    // ─── Token Backspace Edge Cases ──────────────────────────────────────────

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
}
