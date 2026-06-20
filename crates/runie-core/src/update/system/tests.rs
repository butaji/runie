#[test]
fn apply_config_reloads_prompts() {
    let _guard = crate::tests::ENV_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    std::env::set_var("HOME", dir.path());

    let mut state = crate::model::AppState::default();
    let config = crate::config::Config::default();
    state.apply_config(&config);

    assert!(
        !state.prompts.is_empty(),
        "apply_config should load prompts"
    );
    assert_eq!(state.prompts[0].name, "default");
}

#[test]
fn resume_session_opens_session_tree_dialog() {
    let mut state = crate::model::AppState::default();
    state.update(crate::Event::from(
        crate::event::ControlEvent::ResumeSession,
    ));
    assert!(
        matches!(
            state.open_dialog,
            Some(crate::commands::DialogState::SessionTree(_))
        ),
        "ResumeSession should open the session tree dialog, got {:?}",
        state.open_dialog
    );
}

#[test]
fn toggle_vim_mode_marks_dirty() {
    let mut state = crate::model::AppState::default();
    state.view.dirty = false;
    state.update(crate::Event::from(
        crate::event::ControlEvent::ToggleVimMode,
    ));
    assert!(state.view.dirty, "ToggleVimMode should mark the view dirty");
}

#[test]
fn apply_config_switches_active_model_when_not_overridden() {
    let mut state = crate::model::AppState::default();
    state.config.current_provider = "openai".into();
    state.config.current_model = "gpt-4o".into();

    let mut config = crate::config::Config::default();
    config.provider = Some("anthropic".into());
    config.models.default = Some("claude-3".into());
    config.model_providers.insert(
        "anthropic".into(),
        crate::config::ModelProvider {
            provider_type: None,
            base_url: "https://api.anthropic.com".into(),
            api_key: "sk-test".into(),
            models: vec!["claude-3".into()],
        },
    );
    state.apply_config(&config);

    assert_eq!(state.config.current_provider, "anthropic");
    assert_eq!(state.config.current_model, "claude-3");
}

#[test]
fn apply_config_keeps_active_model_when_overridden() {
    let mut state = crate::model::AppState::default();
    state.config.current_provider = "openai".into();
    state.config.current_model = "gpt-4o".into();
    state.config.model_source = crate::state::ModelSource::UserOverride;

    let mut config = crate::config::Config::default();
    config.provider = Some("anthropic".into());
    config.models.default = Some("claude-3".into());
    config.model_providers.insert(
        "anthropic".into(),
        crate::config::ModelProvider {
            provider_type: None,
            base_url: "https://api.anthropic.com".into(),
            api_key: "sk-test".into(),
            models: vec!["claude-3".into()],
        },
    );
    state.apply_config(&config);

    assert_eq!(state.config.current_provider, "openai");
    assert_eq!(state.config.current_model, "gpt-4o");
    assert_eq!(
        state.config.model_source,
        crate::state::ModelSource::UserOverride
    );
}

#[test]
fn set_provider_uses_configured_model_for_custom_provider() {
    let mut state = crate::model::AppState::default();
    state.config.current_provider = "openai".into();
    state.config.current_model = "gpt-4o".into();

    let mut config = crate::config::Config::default();
    config.model_providers.insert(
        "custom".into(),
        crate::config::ModelProvider {
            provider_type: None,
            base_url: "http://test".into(),
            api_key: "key".into(),
            models: vec!["custom-model".into()],
        },
    );
    state.config_cache = Some(config);

    state.set_provider("custom");

    assert_eq!(state.config.current_provider, "custom");
    assert_eq!(state.config.current_model, "custom-model");
}

#[test]
fn apply_config_falls_back_to_first_configured_provider_when_no_default() {
    let mut state = crate::model::AppState::default();
    state.config.current_provider = "openai".into();
    state.config.current_model = "gpt-4o".into();

    let mut config = crate::config::Config::default();
    config.model_providers.insert(
        "custom".into(),
        crate::config::ModelProvider {
            provider_type: None,
            base_url: "http://test".into(),
            api_key: "key".into(),
            models: vec!["custom-model".into()],
        },
    );
    state.apply_config(&config);

    assert_eq!(state.config.current_provider, "custom");
    assert_eq!(state.config.current_model, "custom-model");
    assert_eq!(
        state.config.model_source,
        crate::state::ModelSource::ConfigDefault
    );
}

#[test]
fn apply_config_keeps_override_when_no_default() {
    let mut state = crate::model::AppState::default();
    state.config.current_provider = "openai".into();
    state.config.current_model = "gpt-4o".into();
    state.config.model_source = crate::state::ModelSource::UserOverride;

    let mut config = crate::config::Config::default();
    config.model_providers.insert(
        "custom".into(),
        crate::config::ModelProvider {
            provider_type: None,
            base_url: "http://test".into(),
            api_key: "key".into(),
            models: vec!["custom-model".into()],
        },
    );
    state.apply_config(&config);

    assert_eq!(state.config.current_provider, "openai");
    assert_eq!(state.config.current_model, "gpt-4o");
    assert_eq!(
        state.config.model_source,
        crate::state::ModelSource::UserOverride
    );
}
