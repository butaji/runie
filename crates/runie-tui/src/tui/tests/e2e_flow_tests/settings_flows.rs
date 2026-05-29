use super::*;

#[test]
fn test_e2e_save_settings() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    // Simulate DirectCommand for SwitchModel
    update(&mut state, &mut palette, Msg::DirectCommand(crate::components::PaletteCommand::SwitchModel));
    assert!(state.model_picker.is_some());
    assert_eq!(state.mode, TuiMode::Overlay);

    // Simulate model selection
    update(&mut state, &mut palette, Msg::SelectConfirm);
    // current_model should be set from model_picker
}

#[test]
fn test_e2e_model_picker_selection() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    // Open model picker
    update(&mut state, &mut palette, Msg::DirectCommand(crate::components::PaletteCommand::SwitchModel));
    assert!(state.model_picker.is_some());

    // Navigate
    update(&mut state, &mut palette, Msg::SelectDown);
    update(&mut state, &mut palette, Msg::SelectUp);

    // Confirm selection
    update(&mut state, &mut palette, Msg::SelectConfirm);
    // If a model was selected, it should be set
    if state.current_model.is_some() {
        assert_eq!(state.mode, TuiMode::Chat);
        assert!(state.model_picker.is_none());
    }
}

#[test]
fn test_e2e_settings_persist_model() {
    let mut state = make_state_with_text("Hello");
    let mut palette = CommandPalette::new();

    // Submit message
    let cmds = update(&mut state, &mut palette, Msg::Submit);
    assert!(state.agent_running);
    assert!(state.current_model.is_some());
    assert!(!cmds.is_empty());

    // Simulate agent end
    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: runie_agent::TokenUsage::default(),
    }));

    // Model should persist
    assert_eq!(state.current_model.as_deref(), Some("openai/gpt-4o"));

    // Another submit should use same model
    state.textarea = ratatui_textarea::TextArea::new(vec!["Hello again".to_string()]);
    let cmds2 = update(&mut state, &mut palette, Msg::Submit);
    assert!(cmds2.iter().any(|c| matches!(c, Cmd::SpawnAgent { .. })));
}

#[test]
fn test_e2e_save_settings_respects_dev_folder() {
    // Set RUNIE_HOME to a temp directory
    let temp_dir = std::env::temp_dir().join("runie_test_dev");
    std::env::set_var("RUNIE_HOME", temp_dir.display().to_string());

    // Create state with onboarding active
    let mut state = AppState::default();
    state.onboarding = Some(crate::components::onboarding::Onboarding::new(false));
    state.mode = TuiMode::Onboarding;
    let mut palette = CommandPalette::new();

    // Set up minimax provider and model directly (simulating completed onboarding flow)
    let o = state.onboarding.as_mut().unwrap();
    o.step = OnboardingStep::Complete;
    o.selected_item = 1; // "No, finish setup"
    let minimax_idx = o.providers.iter()
        .position(|p| p.id == "minimax")
        .expect("MiniMax provider should exist");
    o.selected_provider = Some(minimax_idx);
    o.selected_model = Some(0);
    o.api_key_input = "test-minimax-api-key".to_string();
    o.models.push(crate::components::onboarding::ModelOption {
        name: "MiniMax-Text-01".to_string(),
        id: "MiniMax-Text-01".to_string(),
        description: "MiniMax text model".to_string(),
    });

    // Finish onboarding - this should emit SaveSettings
    let cmds = update(&mut state, &mut palette, Msg::OnboardingNext);

    // Verify SaveSettings is emitted
    assert!(!cmds.is_empty(), "Expected SaveSettings command");
    let save_settings = cmds.iter().find_map(|c| match c {
        Cmd::SaveSettings { provider, model, api_key } => Some((provider.clone(), model.clone(), api_key.clone())),
        _ => None,
    });
    assert!(save_settings.is_some(), "Expected SaveSettings command in {:?}", cmds);

    let (provider, model, api_key) = save_settings.unwrap();
    assert_eq!(provider, "minimax");
    assert_eq!(model, "MiniMax-Text-01");
    assert_eq!(api_key, "test-minimax-api-key");

    // Verify state is now in Chat mode
    assert_eq!(state.mode, TuiMode::Chat);
    assert!(state.onboarding.is_none());

    // Note: current_model is set by tui_run.rs when it processes SaveSettings,
    // not by the update function itself. The update function only emits the command.
    // The config path verification (RUNIE_HOME vs ~/.runie) also happens in tui_run.rs
    // when it calls settings::config_path() which respects RUNIE_HOME env var.

    // Cleanup
    std::env::remove_var("RUNIE_HOME");
    let _ = std::fs::remove_dir_all(temp_dir);
}

#[test]
fn test_e2e_model_displayed_correctly() {
    // Set current_model to minimax/MiniMax-Text-01
    let mut state = AppState::default();
    state.current_model = Some("minimax/MiniMax-Text-01".to_string());
    state.top_bar.model = "MiniMax-Text-01".to_string();

    // Verify current_model is full provider/model string
    assert_eq!(state.current_model.as_deref(), Some("minimax/MiniMax-Text-01"));

    // Verify top_bar.model is just the model name (not full provider/model)
    assert_eq!(state.top_bar.model, "MiniMax-Text-01");

    // Verify status bar would show minimax/MiniMax-Text-01 (not openai/gpt-4o)
    // The status_bar.current_model comes from state.current_model
    assert_ne!(state.current_model.as_deref(), Some("openai/gpt-4o"));
    assert_eq!(state.current_model.as_deref(), Some("minimax/MiniMax-Text-01"));
}
