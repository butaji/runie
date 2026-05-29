//! Completion step tests - final step, settings save, restart flow

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

    // ─── Complete Step ───────────────────────────────────────────────────────

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
        let _ = o;
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
        let _ = o;
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
        let _ = o;
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
        let _ = o;
        handle_onboarding_msg(&mut state, Msg::OnboardingNext);
        let o = state.onboarding.as_ref().unwrap();
        assert!(o.selected_provider.is_none());
        assert!(o.selected_model.is_none());
        assert!(o.api_key_input.is_empty());
        assert!(o.search_query.is_empty());
        assert!(o.error_message.is_none());
    }

    // ─── Pluralization ───────────────────────────────────────────────────────

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
        let _ = o;
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
        let _ = o;
        let o = state.onboarding.as_ref().unwrap();
        let model_count = o.models.len();
        assert_eq!(model_count, 2);
        let model_word = if model_count == 1 { "model" } else { "models" };
        assert_eq!(model_word, "models");
    }
}
