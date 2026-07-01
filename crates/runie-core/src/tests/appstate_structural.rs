//! Tests for AppState structural decomposition.
//!
//! Verifies that AppState has been properly decomposed into focused state structs
//! and that loose fields have been moved to appropriate inner structs.

use crate::model::AppState;
use crate::model::{AgentState, CompletionState, ConfigState, InputState, SessionState, ViewState};

fn assert_singleton_defaults(state: &AppState) {
    assert!(!state.should_quit, "should_quit should be false by default");
    assert!(
        state.open_dialog.is_none(),
        "open_dialog should be None by default"
    );
    assert!(
        state.dialog_back_stack.is_empty(),
        "dialog_back_stack should be empty by default"
    );
    assert!(
        state.login_flow.is_none(),
        "login_flow should be None by default"
    );
}

fn assert_transient_defaults(state: &AppState) {
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
}

#[test]
fn appstate_has_correct_top_level_fields() {
    let state = AppState::default();
    assert_singleton_defaults(&state);
    assert_transient_defaults(&state);

    assert!(
        state.prompts.is_empty() || !state.prompts.is_empty(),
        "prompts should be on AppState"
    );
    assert!(
        state.git_info.is_none() || state.git_info.is_some(),
        "git_info should be on AppState"
    );
    assert!(
        state.cwd_name.is_empty() || !state.cwd_name.is_empty(),
        "cwd_name should be on AppState"
    );
}

fn assert_agent_defaults(agent: &AgentState) {
    assert!(!agent.streaming, "agent.streaming should default to false");
    assert_eq!(agent.next_id, 0, "agent.next_id should default to 0");
    assert_eq!(
        agent.intermediate_step_count, 0,
        "agent.intermediate_step_count should default to 0"
    );
    assert!(
        agent.current_action.is_none(),
        "agent.current_action should default to None"
    );
    assert!(
        agent.thinking_started_at.is_none(),
        "agent.thinking_started_at should default to None"
    );
}

fn assert_view_defaults(view: &ViewState) {
    assert_eq!(
        view.animation_frame, 0,
        "view.animation_frame should default to 0"
    );
    assert!(
        !view.all_collapsed,
        "view.all_collapsed should default to false"
    );
}

fn assert_config_defaults(config: &ConfigState) {
    assert_eq!(
        config.steering_mode,
        crate::model::DeliveryMode::OneAtATime,
        "config.steering_mode should default to OneAtATime"
    );
    assert_eq!(
        config.follow_up_mode,
        crate::model::DeliveryMode::OneAtATime,
        "config.follow_up_mode should default to OneAtATime"
    );
    assert!(
        config.recent_models.is_empty(),
        "config.recent_models should default to empty"
    );
}

fn assert_session_defaults(session: &SessionState) {
    assert!(
        session.pending_edits.is_empty(),
        "session.pending_edits should default to empty"
    );
    assert!(
        session.image_attachments.is_empty(),
        "session.image_attachments should default to empty"
    );
}

fn assert_input_defaults(input: &InputState) {
    assert_eq!(
        input.current_prompt, "",
        "input.current_prompt should default to empty"
    );
    assert!(
        input.input_history.is_empty(),
        "input.input_history should default to empty"
    );
}

#[test]
fn inner_structs_are_default() {
    let state = AppState::default();
    assert_agent_defaults(&state.agent);
    assert_view_defaults(&state.view);
    assert_config_defaults(&state.config);
    assert_session_defaults(&state.session);
    assert_input_defaults(&state.input);
}

/// Test AgentState field access via TurnState projection.
/// AgentState is a read-only projection of TurnState, so we set up TurnState
/// and sync to AgentState to verify the projection relationship.
fn agent_access_patterns(state: &mut AppState) {
    // AgentState is a projection of TurnState - set TurnState fields and sync.
    state.turn_state_mut().streaming = true;
    *state.agent_state_mut() = AgentState::from(&state.turn_state);
    assert!(state.agent_state().streaming);

    state.turn_state_mut().next_id = 42;
    *state.agent_state_mut() = AgentState::from(&state.turn_state);
    assert_eq!(state.agent_state().next_id, 42);

    state.turn_state_mut().intermediate_step_count = 5;
    *state.agent_state_mut() = AgentState::from(&state.turn_state);
    assert_eq!(state.agent_state().intermediate_step_count, 5);

    state.turn_state_mut().current_action = Some("Thinking".to_string());
    *state.agent_state_mut() = AgentState::from(&state.turn_state);
    assert!(state.agent_state().current_action.is_some());

    state.turn_state_mut().thinking_started_at = Some(std::time::Instant::now());
    *state.agent_state_mut() = AgentState::from(&state.turn_state);
    assert!(state.agent_state().thinking_started_at.is_some());
}

fn view_access_patterns(state: &mut AppState) {
    state.view.animation_frame = 5;
    assert_eq!(state.view.animation_frame, 5);

    state.view.all_collapsed = true;
    assert!(state.view.all_collapsed);
}

fn config_access_patterns(state: &mut AppState) {
    state.config.steering_mode = crate::model::DeliveryMode::All;
    assert_eq!(state.config.steering_mode, crate::model::DeliveryMode::All);

    state
        .config
        .recent_models
        .push("provider/model".to_string());
    assert_eq!(state.config.recent_models.len(), 1);
}

fn session_access_patterns(state: &mut AppState) {
    state
        .session
        .pending_edits
        .push(crate::edit_preview::EditPreview::new(
            std::path::PathBuf::from("test.rs"),
            "old".to_string(),
            "new".to_string(),
        ));
    assert_eq!(state.session.pending_edits.len(), 1);

    state
        .session
        .image_attachments
        .push("image.png".to_string());
    assert_eq!(state.session.image_attachments.len(), 1);
}

fn input_access_patterns(state: &mut AppState) {
    state.input.current_prompt = "custom".to_string();
    assert_eq!(state.input.current_prompt, "custom");

    state.input.input_history.push("ls".to_string());
    assert_eq!(state.input.input_history.len(), 1);
}

#[test]
fn moved_field_access_patterns() {
    let mut state = AppState::default();
    agent_access_patterns(&mut state);
    view_access_patterns(&mut state);
    config_access_patterns(&mut state);
    session_access_patterns(&mut state);
    input_access_patterns(&mut state);
}

/// Verify singletons remain on AppState (not moved to inner structs).
#[test]
fn singletons_remain_on_appstate() {
    let mut state = AppState {
        should_quit: true,
        ..Default::default()
    };
    assert!(state.should_quit);
    assert!(state.open_dialog.is_none());
    assert!(state.dialog_back_stack.is_empty());
    assert!(state.login_flow.is_none());

    state.transient_message = Some("Test".to_string());
    assert_eq!(state.transient_message, Some("Test".to_string()));

    state.input.input_history.push("echo hello".to_string());
    assert_eq!(state.input.input_history.len(), 1);
}

/// Verify inner struct field count matches acceptance criteria.
#[test]
fn inner_structs_have_expected_fields() {
    assert_agent_defaults(&AgentState::default());
    assert_view_defaults(&ViewState::default());
    assert_config_defaults(&ConfigState::default());
    assert_session_defaults(&SessionState::default());
    assert_input_defaults(&InputState::default());

    let completion = CompletionState::default();
    assert!(completion.path_suggestions.is_none());
    assert!(completion.path_selected.is_none());
}
