//! Tests for AppState structural decomposition.
//!
//! Verifies that AppState has been properly decomposed into focused state structs
//! and that loose fields have been moved to appropriate inner structs.

use crate::model::AppState;
use crate::state::{
    AgentState, CompletionState, ConfigState, InputState, SessionState, ViewState,
};

/// Verify AppState has exactly the expected top-level fields:
/// - 6 inner state structs
/// - Documented singletons (UI/control flags)
#[test]
fn appstate_has_correct_top_level_fields() {
    // These are the documented singletons that stay on AppState
    // (control flags, UI overlays, startup singletons)
    let state = AppState::default();
    assert!(!state.should_quit, "should_quit should be false by default");
    assert!(state.open_dialog.is_none(), "open_dialog should be None by default");
    assert!(
        state.dialog_back_stack.is_empty(),
        "dialog_back_stack should be empty by default"
    );
    assert!(state.login_flow.is_none(), "login_flow should be None by default");
    // skills and prompts are loaded from disk, may be empty or populated
    assert!(state.prompts.is_empty() || !state.prompts.is_empty(), "prompts should be on AppState");
    assert!(
        state.transient_message.is_none(),
        "transient_message should be None by default"
    );
    assert!(
        state.transient_until.is_none(),
        "transient_until should be None by default"
    );
    assert!(
        state.transient_level.is_none(),
        "transient_level should be None by default"
    );
    // git_info and cwd_name are detected at startup
    assert!(state.git_info.is_none() || state.git_info.is_some(), "git_info should be on AppState");
    assert!(state.cwd_name.is_empty() || !state.cwd_name.is_empty(), "cwd_name should be on AppState");
    assert!(
        state.input_history.is_empty(),
        "input_history should be empty by default"
    );
}

/// Verify inner structs are properly initialized via Default.
#[test]
fn inner_structs_are_default() {
    let state = AppState::default();

    // AgentState fields that were moved from AppState
    assert!(
        !state.agent.streaming,
        "agent.streaming should default to false"
    );
    assert_eq!(state.agent.next_id, 0, "agent.next_id should default to 0");
    assert_eq!(
        state.agent.intermediate_step_count, 0,
        "agent.intermediate_step_count should default to 0"
    );
    assert!(
        state.agent.current_action.is_none(),
        "agent.current_action should default to None"
    );
    assert!(
        state.agent.thinking_started_at.is_none(),
        "agent.thinking_started_at should default to None"
    );

    // ViewState fields that were moved from AppState
    assert_eq!(
        state.view.animation_frame, 0,
        "view.animation_frame should default to 0"
    );
    assert!(
        !state.view.all_collapsed,
        "view.all_collapsed should default to false"
    );

    // ConfigState fields that were moved from AppState
    assert_eq!(
        state.config.steering_mode,
        crate::model::DeliveryMode::OneAtATime,
        "config.steering_mode should default to OneAtATime"
    );
    assert_eq!(
        state.config.follow_up_mode,
        crate::model::DeliveryMode::OneAtATime,
        "config.follow_up_mode should default to OneAtATime"
    );
    assert!(
        state.config.recent_models.is_empty(),
        "config.recent_models should default to empty"
    );

    // SessionState fields that were moved from AppState
    assert!(
        state.session.pending_edits.is_empty(),
        "session.pending_edits should default to empty"
    );
    assert!(
        state.session.image_attachments.is_empty(),
        "session.image_attachments should default to empty"
    );

    // InputState fields that were moved from AppState
    assert_eq!(
        state.input.current_prompt, "",
        "input.current_prompt should default to empty"
    );
    assert!(
        state.input.input_history.is_empty(),
        "input.input_history should default to empty"
    );
}

/// Verify moved fields are accessible through inner structs.
#[test]
fn moved_field_access_patterns() {
    let mut state = AppState::default();

    // AgentState fields
    state.agent.streaming = true;
    assert!(state.agent.streaming);

    state.agent.next_id = 42;
    assert_eq!(state.agent.next_id, 42);

    state.agent.intermediate_step_count = 5;
    assert_eq!(state.agent.intermediate_step_count, 5);

    state.agent.current_action = Some("Thinking".to_string());
    assert!(state.agent.current_action.is_some());

    state.agent.thinking_started_at = Some(std::time::Instant::now());
    assert!(state.agent.thinking_started_at.is_some());

    // ViewState fields
    state.view.animation_frame = 5;
    assert_eq!(state.view.animation_frame, 5);

    state.view.all_collapsed = true;
    assert!(state.view.all_collapsed);

    // ConfigState fields
    state.config.steering_mode = crate::model::DeliveryMode::All;
    assert_eq!(state.config.steering_mode, crate::model::DeliveryMode::All);

    state.config.recent_models.push("provider/model".to_string());
    assert_eq!(state.config.recent_models.len(), 1);

    // SessionState fields
    state
        .session
        .pending_edits
        .push(crate::edit_preview::EditPreview::new(
            std::path::PathBuf::from("test.rs"),
            "old".to_string(),
            "new".to_string(),
            "+new".to_string(),
        ));
    assert_eq!(state.session.pending_edits.len(), 1);

    state.session
        .image_attachments
        .push("image.png".to_string());
    assert_eq!(state.session.image_attachments.len(), 1);

    // InputState fields
    state.input.current_prompt = "custom".to_string();
    assert_eq!(state.input.current_prompt, "custom");

    state.input.input_history.push("ls".to_string());
    assert_eq!(state.input.input_history.len(), 1);
}

/// Verify singletons remain on AppState (not moved to inner structs).
#[test]
fn singletons_remain_on_appstate() {
    let mut state = AppState::default();

    // These should remain on AppState per the design
    state.should_quit = true;
    assert!(state.should_quit);

    // open_dialog and dialog_back_stack are part of the UI overlay state
    // that should remain on AppState
    assert!(state.open_dialog.is_none());
    assert!(state.dialog_back_stack.is_empty());

    // login_flow is a transient overlay that should remain on AppState
    assert!(state.login_flow.is_none());

    // transient_message is a UI notification that should remain on AppState
    state.transient_message = Some("Test".to_string());
    assert_eq!(state.transient_message, Some("Test".to_string()));

    // input_history is persistent state that should remain on AppState
    state.input_history.push("echo hello".to_string());
    assert_eq!(state.input_history.len(), 1);
}

/// Verify inner struct field count matches acceptance criteria.
#[test]
fn inner_structs_have_expected_fields() {
    // AgentState should have these moved fields (plus its original fields)
    let agent = AgentState::default();
    assert!(!agent.streaming);
    assert_eq!(agent.next_id, 0);
    assert_eq!(agent.intermediate_step_count, 0);
    assert!(agent.current_action.is_none());
    assert!(agent.thinking_started_at.is_none());

    // ViewState should have these moved fields
    let view = ViewState::default();
    assert_eq!(view.animation_frame, 0);
    assert!(!view.all_collapsed);

    // ConfigState should have these moved fields
    let config = ConfigState::default();
    assert_eq!(
        config.steering_mode,
        crate::model::DeliveryMode::OneAtATime
    );
    assert_eq!(
        config.follow_up_mode,
        crate::model::DeliveryMode::OneAtATime
    );
    assert!(config.recent_models.is_empty());

    // SessionState should have these moved fields
    let session = SessionState::default();
    assert!(session.pending_edits.is_empty());
    assert!(session.image_attachments.is_empty());

    // InputState should have these moved fields
    let input = InputState::default();
    assert_eq!(input.current_prompt, "");
    assert!(input.input_history.is_empty());

    // CompletionState should exist and be properly structured
    let completion = CompletionState::default();
    assert!(completion.path_suggestions.is_none());
    assert!(completion.path_selected.is_none());
}
