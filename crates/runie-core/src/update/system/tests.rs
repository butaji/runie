use super::*;

#[test]
fn reload_all_reloads_skills() {
    let dir = tempfile::tempdir().unwrap();
    std::env::set_var("HOME", dir.path());
    let mut state = crate::model::AppState {
        skills: vec![crate::skills::Skill {
            name: "dummy".into(),
            description: "dummy".into(),
            context: "".into(),
            user_invocable: false,
            file_path: std::path::PathBuf::from("dummy.md"),
        }],
        ..Default::default()
    };
    state.reload_all();
    // In test environment load_all returns empty (no skill dirs exist)
    assert!(
        state.skills.is_empty(),
        "reload_all should reload skills from disk"
    );
    let last = state.session.messages.last().unwrap();
    assert!(
        last.content.contains("Reloaded"),
        "Should confirm reload: {}",
        last.content
    );
    // Prompts should also be reloaded (empty in test env)
    assert!(
        !state.prompts.is_empty(),
        "reload_all should reload prompts"
    );
    assert_eq!(state.prompts[0].name, "default");
}

#[test]
fn resume_session_opens_session_tree_dialog() {
    let mut state = crate::model::AppState::default();
    state.update(crate::Event::from(crate::event::ControlEvent::ResumeSession));
    assert!(
        matches!(state.open_dialog, Some(crate::commands::DialogState::SessionTree(_))),
        "ResumeSession should open the session tree dialog, got {:?}",
        state.open_dialog
    );
}

#[test]
fn toggle_vim_mode_marks_dirty() {
    let mut state = crate::model::AppState::default();
    state.view.dirty = false;
    state.update(crate::Event::from(crate::event::ControlEvent::ToggleVimMode));
    assert!(state.view.dirty, "ToggleVimMode should mark the view dirty");
}

#[test]
fn set_provider_uses_configured_model_for_custom_provider() {
    crate::login_config::set_test_config_with_providers(&[(
        "custom".into(),
        vec!["custom-model".into()],
    )]);
    let mut state = crate::model::AppState::default();
    state.config.current_provider = "openai".into();
    state.config.current_model = "gpt-4o".into();

    state.set_provider("custom");

    assert_eq!(state.config.current_provider, "custom");
    assert_eq!(state.config.current_model, "custom-model");
}
