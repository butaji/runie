use super::*;

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use super::*;

    // ─── Comprehensive Behavioral Tests ─────────────────────────────────────────

    #[test]
    fn test_step_transitions() {
        let mut onboarding = Onboarding::new();

        // Welcome → ProviderSelect
        assert_eq!(onboarding.step, OnboardingStep::Welcome);
        onboarding.next_step();
        assert_eq!(onboarding.step, OnboardingStep::ProviderSelect);

        // ProviderSelect → KeyInput
        onboarding.select_provider(0);
        onboarding.next_step();
        assert_eq!(onboarding.step, OnboardingStep::KeyInput);

        // KeyInput → ModelSelect
        onboarding.api_key_input = "sk-test123".to_string();
        onboarding.next_step();
        assert_eq!(onboarding.step, OnboardingStep::ModelSelect);

        // ModelSelect → Complete
        onboarding.select_model(0);
        onboarding.next_step();
        assert_eq!(onboarding.step, OnboardingStep::Complete);
    }

    #[test]
    fn test_back_navigation() {
        let mut onboarding = Onboarding::new();
        onboarding.step = OnboardingStep::Complete;

        // Complete → ModelSelect
        onboarding.prev_step();
        assert_eq!(onboarding.step, OnboardingStep::ModelSelect);

        // ModelSelect → KeyInput
        onboarding.prev_step();
        assert_eq!(onboarding.step, OnboardingStep::KeyInput);

        // KeyInput → ProviderSelect
        onboarding.prev_step();
        assert_eq!(onboarding.step, OnboardingStep::ProviderSelect);

        // ProviderSelect → Welcome
        onboarding.prev_step();
        assert_eq!(onboarding.step, OnboardingStep::Welcome);

        // Welcome stays at Welcome
        onboarding.prev_step();
        assert_eq!(onboarding.step, OnboardingStep::Welcome);
    }

    #[test]
    fn test_cannot_advance_without_selection() {
        let mut onboarding = Onboarding::new();

        // Welcome → ProviderSelect works
        onboarding.next_step();
        assert_eq!(onboarding.step, OnboardingStep::ProviderSelect);

        // ProviderSelect without selection stays
        onboarding.next_step();
        assert_eq!(onboarding.step, OnboardingStep::ProviderSelect);
        assert!(onboarding.error_message.is_some());

        // Now select a provider to advance to KeyInput
        onboarding.select_provider(0);
        onboarding.next_step();
        assert_eq!(onboarding.step, OnboardingStep::KeyInput);

        // KeyInput without valid key stays (wrong prefix for OpenAI)
        onboarding.api_key_input = "pk-wrong".to_string();
        onboarding.next_step();
        assert_eq!(onboarding.step, OnboardingStep::KeyInput);
        assert!(onboarding.error_message.is_some());

        // Fix key to advance to ModelSelect
        onboarding.api_key_input = "sk-test".to_string();
        onboarding.next_step();
        assert_eq!(onboarding.step, OnboardingStep::ModelSelect);

        // ModelSelect without selection stays
        onboarding.next_step();
        assert_eq!(onboarding.step, OnboardingStep::ModelSelect);
        assert!(onboarding.error_message.is_some());
    }

    #[test]
    fn test_provider_selection_populates_models() {
        let mut onboarding = Onboarding::new();
        assert!(onboarding.models.is_empty());

        // OpenAI
        onboarding.select_provider(0);
        assert_eq!(onboarding.selected_provider, Some(0));
        assert_eq!(onboarding.models.len(), 3);
        assert_eq!(onboarding.models[0].id, "gpt-4o");

        // Anthropic
        onboarding.select_provider(1);
        assert_eq!(onboarding.selected_provider, Some(1));
        assert_eq!(onboarding.models.len(), 3);
        assert_eq!(onboarding.models[0].id, "claude-sonnet-4");

        // Google
        onboarding.select_provider(2);
        assert_eq!(onboarding.selected_provider, Some(2));
        assert_eq!(onboarding.models.len(), 2);
        assert_eq!(onboarding.models[0].id, "gemini-pro");
    }

    #[test]
    fn test_model_selection_clears_on_provider_change() {
        let mut onboarding = Onboarding::new();

        onboarding.select_provider(0);
        onboarding.select_model(1); // GPT-4o Mini
        assert_eq!(onboarding.selected_model, Some(1));

        // Change to Anthropic
        onboarding.select_provider(1);
        assert!(onboarding.selected_model.is_none());

        // Change to Google
        onboarding.select_provider(2);
        assert!(onboarding.selected_model.is_none());
    }

    #[test]
    fn test_key_validation_empty_vs_valid() {
        let mut onboarding = Onboarding::new();
        onboarding.select_provider(0);

        // Empty is invalid
        assert!(!onboarding.validate_key());

        // Whitespace only is invalid
        onboarding.api_key_input = "   ".to_string();
        assert!(!onboarding.validate_key());

        // Valid OpenAI key
        onboarding.api_key_input = "sk-abc123".to_string();
        assert!(onboarding.validate_key());

        // Wrong prefix fails
        onboarding.api_key_input = "pk-abc123".to_string();
        assert!(!onboarding.validate_key());
    }

    #[test]
    fn test_selected_item_navigation_bounds() {
        let mut onboarding = Onboarding::new();
        onboarding.step = OnboardingStep::ProviderSelect;
        assert_eq!(onboarding.selected_item, 0);

        // Up at 0 stays at 0
        onboarding.navigate_up();
        assert_eq!(onboarding.selected_item, 0);

        // Down advances
        onboarding.navigate_down();
        assert_eq!(onboarding.selected_item, 1);

        // Down again
        onboarding.navigate_down();
        assert_eq!(onboarding.selected_item, 2);

        // Down at max stays
        onboarding.navigate_down();
        assert_eq!(onboarding.selected_item, 2);

        // Up goes down
        onboarding.navigate_up();
        assert_eq!(onboarding.selected_item, 1);

        // Up to 0
        onboarding.navigate_up();
        assert_eq!(onboarding.selected_item, 0);

        // Up at 0 stays
        onboarding.navigate_up();
        assert_eq!(onboarding.selected_item, 0);
    }

    #[test]
    fn test_to_settings_returns_none_when_incomplete() {
        let onboarding = Onboarding::new();
        assert!(onboarding.to_settings().is_none());
    }

    #[test]
    fn test_is_complete_checks_all_fields() {
        let mut onboarding = Onboarding::new();
        assert!(!onboarding.is_complete());

        onboarding.select_provider(0);
        assert!(!onboarding.is_complete());

        onboarding.select_model(0);
        assert!(!onboarding.is_complete());

        onboarding.api_key_input = "sk-test".to_string();
        assert!(!onboarding.is_complete());

        onboarding.step = OnboardingStep::Complete;
        assert!(onboarding.is_complete());
    }

    #[test]
    fn test_google_accepts_any_key_format() {
        let mut onboarding = Onboarding::new();
        onboarding.select_provider(2); // Google

        // Empty still invalid
        assert!(!onboarding.validate_key());

        // Any non-empty works
        onboarding.api_key_input = "any-key-format".to_string();
        assert!(onboarding.validate_key());

        onboarding.api_key_input = "AIzaSy...".to_string();
        assert!(onboarding.validate_key());

        onboarding.api_key_input = "not-a-google-key".to_string();
        assert!(onboarding.validate_key());
    }

    // ─── Original Basic Tests ─────────────────────────────────────────────────────

    #[test]
    fn test_onboarding_new() {
        let onboarding = Onboarding::new();
        assert_eq!(onboarding.step, OnboardingStep::Welcome);
        assert_eq!(onboarding.selected_item, 0);
        assert!(onboarding.selected_provider.is_none());
        assert!(onboarding.api_key_input.is_empty());
        assert!(onboarding.selected_model.is_none());
        assert_eq!(onboarding.providers.len(), 3);
    }

    #[test]
    fn test_select_provider() {
        let mut onboarding = Onboarding::new();
        onboarding.select_provider(0); // OpenAI

        assert_eq!(onboarding.selected_provider, Some(0));
        assert_eq!(onboarding.models.len(), 3);
        assert_eq!(onboarding.models[0].id, "gpt-4o");
    }

    #[test]
    fn test_validate_key_openai() {
        let mut onboarding = Onboarding::new();
        onboarding.select_provider(0); // OpenAI
        onboarding.api_key_input = "sk-abc123".to_string();

        assert!(onboarding.validate_key());
    }

    #[test]
    fn test_validate_key_openai_wrong_prefix() {
        let mut onboarding = Onboarding::new();
        onboarding.select_provider(0); // OpenAI
        onboarding.api_key_input = "pk-abc123".to_string();

        assert!(!onboarding.validate_key());
    }

    #[test]
    fn test_validate_key_anthropic() {
        let mut onboarding = Onboarding::new();
        onboarding.select_provider(1); // Anthropic
        onboarding.api_key_input = "sk-ant-api123".to_string();

        assert!(onboarding.validate_key());
    }

    #[test]
    fn test_validate_key_google() {
        let mut onboarding = Onboarding::new();
        onboarding.select_provider(2); // Google
        onboarding.api_key_input = "any-key-format".to_string();

        assert!(onboarding.validate_key());
    }

    #[test]
    fn test_navigate_up_down() {
        let mut onboarding = Onboarding::new();
        onboarding.step = OnboardingStep::ProviderSelect;

        // Initially at 0
        assert_eq!(onboarding.selected_item, 0);

        // Navigate down
        onboarding.navigate_down();
        assert_eq!(onboarding.selected_item, 1);

        onboarding.navigate_down();
        assert_eq!(onboarding.selected_item, 2);

        // At max (providers.len() - 1 = 2), shouldn't go further
        onboarding.navigate_down();
        assert_eq!(onboarding.selected_item, 2);

        // Navigate up
        onboarding.navigate_up();
        assert_eq!(onboarding.selected_item, 1);

        onboarding.navigate_up();
        assert_eq!(onboarding.selected_item, 0);

        // At 0, shouldn't go further
        onboarding.navigate_up();
        assert_eq!(onboarding.selected_item, 0);
    }

    #[test]
    fn test_to_settings() {
        let mut onboarding = Onboarding::new();
        onboarding.select_provider(0); // OpenAI
        onboarding.select_model(0); // GPT-4o
        onboarding.api_key_input = "sk-test123".to_string();
        onboarding.step = OnboardingStep::Complete;

        let settings = onboarding.to_settings().unwrap();
        assert_eq!(settings.provider_id, "openai");
        assert_eq!(settings.model_id, "gpt-4o");
        assert_eq!(settings.api_key, "sk-test123");
    }
}
