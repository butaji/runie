//! Scenario / flow validation tests for the API-gated login flow.
//!
//! The user cannot reach the model selector until the API key has been
//! successfully verified by a `/models` response.

#[cfg(test)]
mod tests {
    use crate::commands::DialogKind;
    use crate::login_flow::state::LoginStep;
    use crate::model::AppState;
    use crate::Event;

    // -----------------------------------------------------------------------
    // Layer 2 helpers
    // -----------------------------------------------------------------------

    fn drive_to_model_select(provider: &str) -> AppState {
        let mut state = AppState::default();
        state.update(Event::Start);
        state.update(Event::SelectProvider { provider: provider.into() });
        let defaults: Vec<String> = crate::provider::find_provider(provider)
            .map(|m| {
                m.models
                    .iter()
                    .map(|model| model.name.to_string())
                    .collect()
            })
            .unwrap_or_default();
        state.update(Event::SubmitKey { provider: provider.into(), key: "sk-test".into() });
        assert_flow_step(&state, LoginStep::Validating);
        state.update(Event::ModelsFetched {
            provider: provider.into(),
            key: "sk-test".into(),
            models: defaults.clone(),
        });
        let flow = state.login_flow().unwrap();
        assert_eq!(flow.step, LoginStep::ModelSelect);
        assert_eq!(flow.available_models, defaults);
        assert!(flow.selected_models.len() == defaults.len());
        state
    }

    fn assert_flow_step(state: &AppState, step: LoginStep) {
        assert_eq!(state.login_flow().unwrap().step, step);
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
    // Layer 2: open / cancel / navigation
    // -----------------------------------------------------------------------

    #[test]
    fn key_input_esc_pops_to_provider_picker_not_close() {
        let mut state = AppState::default();
        state.update(Event::Start);
        state.update(Event::SelectProvider { provider: "minimax".into() });
        match &state.open_dialog_mut() {
            Some(crate::commands::DialogState::Active { kind: DialogKind::Generic, panels: s }) => {
                assert_eq!(s.len(), 2, "stack should be [provider, key_input]");
                assert_eq!(s.current().unwrap().id, "login-key");
            }
            other => panic!("expected PanelStack, got {other:?}"),
        }
        state.update(Event::dialog_back());
        match &state.open_dialog_mut() {
            Some(crate::commands::DialogState::Active { kind: DialogKind::Generic, panels: s }) => {
                assert_eq!(s.len(), 1, "Esc must pop to root, not close");
                assert_eq!(s.current().unwrap().id, "login-provider");
            }
            other => panic!("Esc on key input must leave dialog open at root, got {other:?}"),
        }
    }

    #[test]
    fn login_command_opens_provider_picker() {
        let mut state = AppState::default();
        state.update(Event::Start);
        assert!(state.open_dialog().is_some());
        assert!(state.login_flow().is_some());
        assert_flow_step(&state, LoginStep::ProviderPicker);
    }

    #[test]
    fn login_select_provider_pushes_key_input() {
        let mut state = AppState::default();
        state.update(Event::Start);
        state.update(Event::SelectProvider { provider: "minimax".into() });
        assert_flow_step(&state, LoginStep::KeyInput);
        assert_eq!(state.login_flow().unwrap().provider, "minimax");
    }

    #[test]
    fn login_switching_providers_clears_previous_key() {
        let mut state = AppState::default();
        state.update(Event::Start);
        state.update(Event::SelectProvider { provider: "moonshotai".into() });
        state.update(Event::SubmitKey { provider: "moonshotai".into(), key: "sk-secret".into() });
        assert_eq!(state.login_flow().unwrap().key, "sk-secret");

        state.update(Event::SelectProvider { provider: "deepseek".into() });
        assert!(
            state.login_flow().unwrap().key.is_empty(),
            "switching providers must not carry the previous provider's key \
             into the new login form (leaked credentials across services)"
        );
    }

    #[test]
    fn login_reselecting_same_provider_keeps_key() {
        let mut state = AppState::default();
        state.update(Event::Start);
        state.update(Event::SelectProvider { provider: "moonshotai".into() });
        state.update(Event::SubmitKey { provider: "moonshotai".into(), key: "sk-secret".into() });

        state.update(Event::SelectProvider { provider: "moonshotai".into() });
        assert_eq!(
            state.login_flow().unwrap().key,
            "sk-secret",
            "re-selecting the same provider (back navigation) should keep the typed key"
        );
    }

    #[test]
    fn login_submit_key_goes_to_validating() {
        let mut state = AppState::default();
        state.update(Event::Start);
        state.update(Event::SelectProvider { provider: "minimax".into() });
        state.update(Event::SubmitKey { provider: "minimax".into(), key: "sk-test".into() });
        assert_flow_step(&state, LoginStep::Validating);
        match &state.open_dialog_mut() {
            Some(crate::commands::DialogState::Active { kind: DialogKind::Generic, panels: s }) => {
                assert_eq!(s.current().unwrap().id, "login-validating");
            }
            other => panic!("expected validating panel, got {other:?}"),
        }
    }

    #[test]
    fn login_submit_key_preserves_provider_when_empty() {
        let mut state = AppState::default();
        state.update(Event::Start);
        state.update(Event::SelectProvider { provider: "minimax".into() });
        state.update(Event::SubmitKey { provider: "".into(), key: "sk-test".into() });
        assert_flow_step(&state, LoginStep::Validating);
        state.update(Event::ModelsFetched {
            provider: "minimax".into(),
            key: "sk-test".into(),
            models: vec!["MiniMax-M3".into()],
        });
        let flow = state.login_flow().unwrap();
        assert_eq!(flow.step, LoginStep::ModelSelect);
        assert_eq!(flow.provider, "minimax");
        assert_eq!(flow.key, "sk-test");
    }

    #[test]
    fn login_validation_failure_returns_to_key_input() {
        let mut state = AppState::default();
        state.update(Event::Start);
        state.update(Event::SelectProvider { provider: "minimax".into() });
        state.update(Event::SubmitKey { provider: "minimax".into(), key: "sk-bad".into() });
        assert_flow_step(&state, LoginStep::Validating);
        state.update(Event::ValidationFailed {
            provider: "minimax".into(),
            key: "sk-bad".into(),
            error: "unauthorized".into(),
        });
        assert_flow_step(&state, LoginStep::KeyInput);
        assert_transient_contains(&state, "Could not verify key");
    }

    #[test]
    fn login_toggle_model_updates_selection() {
        let mut state = drive_to_model_select("minimax");
        let first = state.login_flow().unwrap().available_models[0].clone();
        assert!(state
            .login_flow
            .as_ref()
            .unwrap()
            .selected_models
            .contains(&first));
        state.update(Event::ToggleModel { model: first.clone() });
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
        state.update(Event::Start);
        state.update(Event::Cancel);
        assert!(state.open_dialog().is_none());
        assert!(state.login_flow().is_none());
    }

    // -----------------------------------------------------------------------
    // Layer 2: validation-gated scenarios
    // -----------------------------------------------------------------------

    #[test]
    fn s1_submit_key_shows_validating_panel() {
        let mut state = AppState::default();
        state.update(Event::Start);
        state.update(Event::SelectProvider { provider: "minimax".into() });
        state.update(Event::SubmitKey { provider: "minimax".into(), key: "sk-test".into() });
        assert_flow_step(&state, LoginStep::Validating);
    }

    #[test]
    fn s1_models_fetched_reaches_model_select() {
        let mut state = AppState::default();
        state.update(Event::Start);
        state.update(Event::SelectProvider { provider: "minimax".into() });
        state.update(Event::SubmitKey { provider: "minimax".into(), key: "sk-test".into() });
        state.update(Event::ModelsFetched {
            provider: "minimax".into(),
            key: "sk-test".into(),
            models: vec!["new-A".into(), "new-B".into()],
        });
        let flow = state.login_flow().unwrap();
        assert_eq!(flow.step, LoginStep::ModelSelect);
        assert_eq!(flow.available_models, vec!["new-A", "new-B"]);
        assert!(flow.selected_models.contains("new-A"));
        assert!(flow.selected_models.contains("new-B"));
        assert!(flow.validated);
    }

    #[test]
    fn s3_validation_failed_returns_to_key_input() {
        let mut state = AppState::default();
        state.update(Event::Start);
        state.update(Event::SelectProvider { provider: "minimax".into() });
        state.update(Event::SubmitKey { provider: "minimax".into(), key: "sk-test".into() });
        state.update(Event::ValidationFailed {
            provider: "minimax".into(),
            key: "sk-test".into(),
            error: "connection refused".into(),
        });
        assert_flow_step(&state, LoginStep::KeyInput);
        assert_transient_contains(&state, "verify");
    }

    #[test]
    fn s4_invalid_key_shows_transient_not_error_panel() {
        let mut state = AppState::default();
        state.update(Event::Start);
        state.update(Event::SelectProvider { provider: "minimax".into() });
        state.update(Event::SubmitKey { provider: "minimax".into(), key: "sk-test".into() });
        state.update(Event::ValidationFailed {
            provider: "minimax".into(),
            key: "sk-test".into(),
            error: "API validation failed: 401 Unauthorized".into(),
        });
        assert_flow_step(&state, LoginStep::KeyInput);
        assert!(state.transient_message().is_some());
    }

    #[test]
    fn s6_save_before_validation_is_rejected() {
        let mut state = AppState::default();
        state.update(Event::Start);
        state.update(Event::SelectProvider { provider: "minimax".into() });
        state.update(Event::SubmitKey { provider: "minimax".into(), key: "sk-test".into() });
        assert_flow_step(&state, LoginStep::Validating);
        state.update(Event::Save);
        assert!(
            state.login_flow().is_some(),
            "save should be rejected before validation"
        );
        assert_transient_contains(&state, "validated");
    }

    #[test]
    fn google_models_fetched_filters_to_curated_chat_models() {
        // Gemini's /models endpoint returns the whole Google model zoo —
        // video, image, embedding, robotics, TTS — most of which cannot chat.
        // Onboarding must offer only the curated chat models from the
        // registry, in registry order, so the default lands on a live model.
        let mut state = AppState::default();
        state.update(Event::Start);
        state.update(Event::SelectProvider { provider: "google".into() });
        state.update(Event::SubmitKey { provider: "google".into(), key: "test".into() });
        state.update(Event::ModelsFetched {
            provider: "google".into(),
            key: "test".into(),
            models: vec![
                "veo-3.1-fast-generate-preview".into(),
                "gemini-3.1-pro-preview".into(),
                "imagen-4.0-generate-001".into(),
                "gemini-3.1-flash-lite".into(),
            ],
        });
        let flow = state.login_flow().unwrap();
        assert_eq!(
            flow.available_models,
            vec!["gemini-3.1-flash-lite", "gemini-3.1-pro-preview"]
        );
        assert!(!flow
            .selected_models
            .contains("veo-3.1-fast-generate-preview"));
        assert!(!flow.selected_models.contains("imagen-4.0-generate-001"));
    }

    #[test]
    fn google_models_fetched_keeps_raw_list_when_no_registry_match() {
        // Registry drift must not brick onboarding: when none of the fetched
        // models are in the curated list, show the raw fetched list.
        let mut state = AppState::default();
        state.update(Event::Start);
        state.update(Event::SelectProvider { provider: "google".into() });
        state.update(Event::SubmitKey { provider: "google".into(), key: "test".into() });
        state.update(Event::ModelsFetched {
            provider: "google".into(),
            key: "test".into(),
            models: vec!["future-gemini-9".into()],
        });
        let flow = state.login_flow().unwrap();
        assert_eq!(flow.available_models, vec!["future-gemini-9"]);
    }

    #[test]
    fn s8_empty_fetch_reaches_model_select_with_no_models() {
        let mut state = AppState::default();
        state.update(Event::Start);
        state.update(Event::SelectProvider { provider: "minimax".into() });
        state.update(Event::SubmitKey { provider: "minimax".into(), key: "sk-test".into() });
        state.update(Event::ModelsFetched { provider: "minimax".into(), key: "sk-test".into(), models: vec![] });
        let flow = state.login_flow().unwrap();
        assert_eq!(flow.step, LoginStep::ModelSelect);
        assert!(flow.available_models.is_empty());
        assert!(flow.selected_models.is_empty());
    }

    #[test]
    fn s9_unknown_provider_no_defaults() {
        let mut state = AppState::default();
        state.update(Event::Start);
        state.update(Event::SelectProvider { provider: "ghost".into() });
        state.update(Event::SubmitKey { provider: "ghost".into(), key: "k".into() });
        let login_flow = state.login_flow();
        let flow = login_flow.as_ref().unwrap();
        assert_eq!(flow.step, LoginStep::Validating);
        assert!(flow.available_models.is_empty());
    }

    #[test]
    fn s13_empty_key_is_rejected() {
        let mut state = AppState::default();
        state.update(Event::Start);
        state.update(Event::SelectProvider { provider: "minimax".into() });
        state.update(Event::SubmitKey { provider: "minimax".into(), key: "".into() });
        let login_flow = state.login_flow();
        let flow = login_flow.as_ref().unwrap();
        assert_eq!(flow.step, LoginStep::KeyInput);
        assert_transient_contains(&state, "API key is required");
    }
}
