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
        onboarding.update_search("");
        assert_eq!(onboarding.filtered_provider_indices.len(), 21);

        // Select OpenAI (index 15 in filtered list after alphabetical sort)
        onboarding.select_provider(15);
        onboarding.next_step();
        assert_eq!(onboarding.step, OnboardingStep::KeyInput);

        // KeyInput → ModelSelect
        onboarding.api_key_input = "sk-test123".to_string();
        onboarding.next_step();
        assert_eq!(onboarding.step, OnboardingStep::ModelSelect);

        // ModelSelect → Complete
        onboarding.update_search(""); // populate filtered_model_indices
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

        // Initialize search to populate filtered_provider_indices
        onboarding.update_search("");

        // ProviderSelect without selection stays
        onboarding.next_step();
        assert_eq!(onboarding.step, OnboardingStep::ProviderSelect);
        assert!(onboarding.error_message.is_some());

        // Now select a provider to advance to KeyInput
        onboarding.select_provider(15); // OpenAI
        onboarding.next_step();
        assert_eq!(onboarding.step, OnboardingStep::KeyInput);

        // KeyInput without valid key stays
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
        onboarding.step = OnboardingStep::ProviderSelect;
        onboarding.update_search(""); // Populate filtered_provider_indices

        assert!(onboarding.models.is_empty());

        // OpenAI (index 15 after alphabetical sort)
        onboarding.select_provider(15);
        assert_eq!(onboarding.selected_provider, Some(15));
        assert_eq!(onboarding.models.len(), 3);
        // Models sorted alphabetically: "GPT-4o" < "GPT-4o Mini" < "O1 Mini"
        assert_eq!(onboarding.models[0].id, "gpt-4o");         // GPT-4o
        assert_eq!(onboarding.models[1].id, "gpt-4o-mini");   // GPT-4o Mini
        assert_eq!(onboarding.models[2].id, "o1-mini");        // O1 Mini

        // Anthropic (index 0)
        onboarding.select_provider(0);
        assert_eq!(onboarding.selected_provider, Some(0));
        assert_eq!(onboarding.models.len(), 3);
        // Models sorted: Claude Haiku < Claude Opus < Claude Sonnet 4
        assert_eq!(onboarding.models[0].id, "claude-haiku");
        assert_eq!(onboarding.models[1].id, "claude-opus");
        assert_eq!(onboarding.models[2].id, "claude-sonnet-4");

        // Google (index 5)
        onboarding.select_provider(5);
        assert_eq!(onboarding.selected_provider, Some(5));
        assert_eq!(onboarding.models.len(), 2);
        // Models sorted: Gemini Flash < Gemini Pro
        assert_eq!(onboarding.models[0].id, "gemini-flash");
        assert_eq!(onboarding.models[1].id, "gemini-pro");
    }

    #[test]
    fn test_model_selection_clears_on_provider_change() {
        let mut onboarding = Onboarding::new();
        onboarding.step = OnboardingStep::ProviderSelect;

        // Initialize search to populate filtered_provider_indices
        onboarding.update_search("");

        // OpenAI (index 15)
        onboarding.select_provider(15);
        onboarding.step = OnboardingStep::ModelSelect;
        onboarding.update_search(""); // Populate filtered_model_indices
        onboarding.select_model(1); // GPT-4o Mini
        assert_eq!(onboarding.selected_model, Some(1));

        // Change to Anthropic (index 0)
        onboarding.step = OnboardingStep::ProviderSelect;
        onboarding.select_provider(0);
        assert!(onboarding.selected_model.is_none());

        // Change to Google (index 5)
        onboarding.select_provider(5);
        assert!(onboarding.selected_model.is_none());
    }

    #[test]
    fn test_key_validation_empty_vs_valid() {
        let mut onboarding = Onboarding::new();
        onboarding.step = OnboardingStep::ProviderSelect;
        onboarding.update_search(""); // Populate filtered_provider_indices
        onboarding.select_provider(15); // OpenAI

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
        onboarding.update_search(""); // Populate filtered_provider_indices
        let max_index = onboarding.providers.len() - 1; // 20
        assert_eq!(onboarding.selected_item, 0);

        // Up at 0 stays at 0
        onboarding.navigate_up();
        assert_eq!(onboarding.selected_item, 0);

        // Navigate down to max (20)
        for _ in 0..max_index {
            onboarding.navigate_down();
        }
        assert_eq!(onboarding.selected_item, max_index);

        // Down at max stays
        onboarding.navigate_down();
        assert_eq!(onboarding.selected_item, max_index);

        // Navigate up back to 0
        for _ in 0..max_index {
            onboarding.navigate_up();
        }
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
        onboarding.step = OnboardingStep::ProviderSelect;
        onboarding.update_search(""); // Populate filtered_provider_indices

        assert!(!onboarding.is_complete());

        onboarding.select_provider(0);
        assert!(!onboarding.is_complete());

        onboarding.step = OnboardingStep::ModelSelect;
        onboarding.update_search(""); // Populate filtered_model_indices
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
        onboarding.step = OnboardingStep::ProviderSelect;
        onboarding.update_search(""); // Populate filtered_provider_indices
        onboarding.select_provider(5); // Google (index 5 after alphabetical sort)

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
        assert_eq!(onboarding.providers.len(), 21);
    }

    #[test]
    fn test_select_provider() {
        let mut onboarding = Onboarding::new();
        onboarding.step = OnboardingStep::ProviderSelect;
        onboarding.update_search(""); // Populate filtered_provider_indices
        onboarding.select_provider(15); // OpenAI (index 15 after alphabetical sort)

        assert_eq!(onboarding.selected_provider, Some(15));
        assert_eq!(onboarding.models.len(), 3);
        // Models sorted: GPT-4o < GPT-4o Mini < O1 Mini
        assert_eq!(onboarding.models[0].id, "gpt-4o"); // GPT-4o
    }

    #[test]
    fn test_validate_key_openai() {
        let mut onboarding = Onboarding::new();
        onboarding.step = OnboardingStep::ProviderSelect;
        onboarding.update_search(""); // Populate filtered_provider_indices
        onboarding.select_provider(15); // OpenAI
        onboarding.api_key_input = "sk-abc123".to_string();

        assert!(onboarding.validate_key());
    }

    #[test]
    fn test_validate_key_openai_wrong_prefix() {
        let mut onboarding = Onboarding::new();
        onboarding.step = OnboardingStep::ProviderSelect;
        onboarding.update_search(""); // Populate filtered_provider_indices
        onboarding.select_provider(15); // OpenAI
        onboarding.api_key_input = "pk-abc123".to_string();

        assert!(!onboarding.validate_key());
    }

    #[test]
    fn test_validate_key_anthropic() {
        let mut onboarding = Onboarding::new();
        onboarding.step = OnboardingStep::ProviderSelect;
        onboarding.update_search(""); // Populate filtered_provider_indices
        onboarding.select_provider(0); // Anthropic (index 0 after alphabetical sort)
        onboarding.api_key_input = "sk-ant-api123".to_string();

        assert!(onboarding.validate_key());
    }

    #[test]
    fn test_validate_key_google() {
        let mut onboarding = Onboarding::new();
        onboarding.step = OnboardingStep::ProviderSelect;
        onboarding.update_search(""); // Populate filtered_provider_indices
        onboarding.select_provider(5); // Google (index 5 after alphabetical sort)
        onboarding.api_key_input = "any-key-format".to_string();

        assert!(onboarding.validate_key());
    }

    #[test]
    fn test_navigate_up_down() {
        let mut onboarding = Onboarding::new();
        onboarding.step = OnboardingStep::ProviderSelect;
        onboarding.update_search(""); // Populate filtered_provider_indices
        let max_index = onboarding.providers.len() - 1; // 20

        // Initially at 0
        assert_eq!(onboarding.selected_item, 0);

        // Navigate down to max (20)
        for _ in 0..max_index {
            onboarding.navigate_down();
        }
        assert_eq!(onboarding.selected_item, max_index);

        // At max, shouldn't go further
        onboarding.navigate_down();
        assert_eq!(onboarding.selected_item, max_index);

        // Navigate up back to 0
        for _ in 0..max_index {
            onboarding.navigate_up();
        }
        assert_eq!(onboarding.selected_item, 0);

        // At 0, shouldn't go further
        onboarding.navigate_up();
        assert_eq!(onboarding.selected_item, 0);
    }

    #[test]
    fn test_to_settings() {
        let mut onboarding = Onboarding::new();
        onboarding.step = OnboardingStep::ProviderSelect;
        onboarding.update_search(""); // Populate filtered_provider_indices
        onboarding.select_provider(15); // OpenAI (index 15 after alphabetical sort)

        onboarding.step = OnboardingStep::ModelSelect;
        onboarding.update_search(""); // Populate filtered_model_indices
        onboarding.select_model(0); // GPT-4o (first alphabetically)
        onboarding.api_key_input = "sk-test123".to_string();
        onboarding.step = OnboardingStep::Complete;

        let settings = onboarding.to_settings().unwrap();
        assert_eq!(settings.provider_id, "openai");
        assert_eq!(settings.model_id, "gpt-4o"); // GPT-4o is first alphabetically
        assert_eq!(settings.api_key, "sk-test123");
    }

    // ─── Fuzzy Search Tests ──────────────────────────────────────────────────────

    #[test]
    fn test_fuzzy_search_providers() {
        let mut onboarding = Onboarding::new();
        onboarding.step = OnboardingStep::ProviderSelect;

        // Search for "ope" - should match OpenAI and OpenRouter
        onboarding.update_search("ope");
        assert_eq!(onboarding.search_query, "ope");
        assert_eq!(onboarding.filtered_provider_indices.len(), 2);

        // Verify the filtered providers contain OpenAI and OpenRouter
        let filtered_providers: Vec<&str> = onboarding.filtered_provider_indices
            .iter()
            .map(|&i| onboarding.providers[i].name.as_str())
            .collect();
        assert!(filtered_providers.contains(&"OpenAI"));
        assert!(filtered_providers.contains(&"OpenRouter"));
    }

    #[test]
    fn test_fuzzy_search_models() {
        let mut onboarding = Onboarding::new();
        onboarding.step = OnboardingStep::ProviderSelect;
        onboarding.update_search(""); // Populate filtered_provider_indices
        onboarding.select_provider(15); // OpenAI
        onboarding.step = OnboardingStep::ModelSelect;
        onboarding.update_search(""); // Populate filtered_model_indices

        // Search for "mini" - should match GPT-4o Mini and O1 Mini
        onboarding.update_search("mini");
        assert_eq!(onboarding.search_query, "mini");
        assert_eq!(onboarding.filtered_model_indices.len(), 2);

        // Verify the filtered models contain GPT-4o Mini and O1 Mini
        let filtered_models: Vec<&str> = onboarding.filtered_model_indices
            .iter()
            .map(|&i| onboarding.models[i].name.as_str())
            .collect();
        assert!(filtered_models.contains(&"GPT-4o Mini"));
        assert!(filtered_models.contains(&"O1 Mini"));
    }

    #[test]
    fn test_search_no_matches() {
        let mut onboarding = Onboarding::new();
        onboarding.step = OnboardingStep::ProviderSelect;

        // Search for "xyz" - no matches
        onboarding.update_search("xyz");
        assert_eq!(onboarding.filtered_provider_indices.len(), 0);
    }

    #[test]
    fn test_clear_search() {
        let mut onboarding = Onboarding::new();
        onboarding.step = OnboardingStep::ProviderSelect;

        // Full list
        assert_eq!(onboarding.get_filtered_provider_count(), 21);

        // Search for "ope" - reduces to 2
        onboarding.update_search("ope");
        assert_eq!(onboarding.get_filtered_provider_count(), 2);

        // Clear search - full list restored
        onboarding.clear_search();
        assert!(onboarding.search_query.is_empty());
        assert!(onboarding.filtered_provider_indices.is_empty());
        assert_eq!(onboarding.get_filtered_provider_count(), 21);
    }

    // ─── Navigation with Filtered List Tests ───────────────────────────────────

    #[test]
    fn test_navigation_bounds_with_filtered_list() {
        let mut onboarding = Onboarding::new();
        onboarding.step = OnboardingStep::ProviderSelect;

        // Search for "ope" - filters to OpenAI (sorted index 15) and OpenRouter (sorted index 16)
        onboarding.update_search("ope");
        assert_eq!(onboarding.get_filtered_provider_count(), 2);
        assert_eq!(onboarding.selected_item, 0);

        // Max index should be 1 (len - 1 = 2 - 1 = 1)
        let max_index = onboarding.get_filtered_provider_count() - 1;
        assert_eq!(max_index, 1);

        // Navigate down to max
        onboarding.navigate_down();
        assert_eq!(onboarding.selected_item, 1);

        // Down at max stays
        onboarding.navigate_down();
        assert_eq!(onboarding.selected_item, 1);

        // Navigate up back to 0
        onboarding.navigate_up();
        assert_eq!(onboarding.selected_item, 0);

        // Up at 0 stays
        onboarding.navigate_up();
        assert_eq!(onboarding.selected_item, 0);
    }
}
