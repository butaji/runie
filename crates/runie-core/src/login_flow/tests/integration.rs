//! Login flow event-handling integration tests.

use crate::login_flow::LoginStep;
use crate::model::AppState;
use crate::Event;

// Layer 2: event handling through AppState
// -----------------------------------------------------------------------

/// Helper: drive the login flow to the model selector with the given
/// provider and a known-defaults key. Returns the resulting state.
fn drive_to_model_select(provider: &str) -> AppState {
    let mut state = AppState::default();
    state.update(Event::LoginFlowStart);
    state.update(Event::LoginFlowSelectProvider {
        provider: provider.into(),
    });
    let defaults = crate::provider_registry::find_provider(provider)
        .map(|m| m.default_models.to_vec())
        .unwrap_or_default();
    state.update(Event::LoginFlowSubmitKey {
        provider: provider.into(),
        key: "sk-test".into(),
    });
    // SubmitKey handler must populate defaults itself.
    let flow = state.login_flow.as_ref().unwrap();
    assert_eq!(flow.step, LoginStep::ModelSelect);
    assert_eq!(flow.available_models, defaults);
    assert!(flow.selected_models.len() == defaults.len());
    state
}

// S1: happy path, network fast
#[test]
fn s1_submit_key_immediately_shows_defaults() {
    let state = drive_to_model_select("minimax");
    let flow = state.login_flow.as_ref().unwrap();
    assert_eq!(flow.step, LoginStep::ModelSelect);
    assert!(!flow.available_models.is_empty());
}

// S1 continued: background fetch success replaces the list
#[test]
fn s1_models_fetched_event_replaces_list() {
    let mut state = drive_to_model_select("minimax");
    state.update(Event::LoginFlowModelsFetched {
        provider: "minimax".into(),
        key: "sk-test".into(),
        models: vec!["new-A".into(), "new-B".into()],
    });
    let flow = state.login_flow.as_ref().unwrap();
    assert_eq!(flow.step, LoginStep::ModelSelect);
    assert_eq!(flow.available_models, vec!["new-A", "new-B"]);
    assert!(flow.selected_models.contains("new-A"));
    assert!(flow.selected_models.contains("new-B"));
}

// S2: happy path, slow network — defaults shown first, then replaced
#[test]
fn s2_slow_fetch_user_can_toggle_before_it_returns() {
    let mut state = drive_to_model_select("minimax");
    // User deselects one of the defaults
    let first_default = state.login_flow.as_ref().unwrap().available_models[0].clone();
    state.update(Event::LoginFlowToggleModel {
        model: first_default.clone(),
    });
    assert!(!state
        .login_flow
        .as_ref()
        .unwrap()
        .selected_models
        .contains(&first_default));

    // Fetch returns later with the same list
    let defaults = state.login_flow.as_ref().unwrap().available_models.clone();
    state.update(Event::LoginFlowModelsFetched {
        provider: "minimax".into(),
        key: "sk-test".into(),
        models: defaults.clone(),
    });
    let flow = state.login_flow.as_ref().unwrap();
    // The user's deselect is preserved.
    assert!(!flow.selected_models.contains(&first_default));
    for m in &defaults {
        if m != &first_default {
            assert!(flow.selected_models.contains(m));
        }
    }
}

// S3: network unreachable / timeout — transient warning, step unchanged
#[test]
fn s3_validation_failed_does_not_block_user() {
    let mut state = drive_to_model_select("minimax");
    state.update(Event::LoginFlowValidationFailed {
        provider: "minimax".into(),
        key: "sk-test".into(),
        error: "connection refused".into(),
    });
    let flow = state.login_flow.as_ref().unwrap();
    // Step must remain ModelSelect — the user is NOT pushed to an error panel.
    assert_eq!(flow.step, LoginStep::ModelSelect);
    // The transient message is set.
    let transient = state.transient_message.as_ref();
    assert!(transient.is_some(), "transient warning should be set");
    let content = transient.unwrap();
    assert!(content.contains("verify") || content.contains("refused"));
    // Defaults are still available.
    assert!(!flow.available_models.is_empty());
}

// S4: invalid key (401) — same as S3, non-blocking
#[test]
fn s4_invalid_key_shows_transient_not_error_panel() {
    let mut state = drive_to_model_select("minimax");
    state.update(Event::LoginFlowValidationFailed {
        provider: "minimax".into(),
        key: "sk-test".into(),
        error: "API validation failed: 401 Unauthorized".into(),
    });
    let flow = state.login_flow.as_ref().unwrap();
    assert_eq!(flow.step, LoginStep::ModelSelect);
    assert!(state.transient_message.is_some());
}

// S5: fetch result after user toggle (covered in s2; here we assert
// that the new models from the fetch are auto-selected while the
// user's deselect on an existing one sticks).
#[test]
fn s5_fetch_superset_preserves_toggle_and_selects_new() {
    let mut state = drive_to_model_select("minimax");
    let first = state.login_flow.as_ref().unwrap().available_models[0].clone();
    state.update(Event::LoginFlowToggleModel {
        model: first.clone(),
    });
    // Fetch returns the existing list plus a brand-new model.
    let mut new_list = state.login_flow.as_ref().unwrap().available_models.clone();
    new_list.push("brand-new-model".into());
    state.update(Event::LoginFlowModelsFetched {
        provider: "minimax".into(),
        key: "sk-test".into(),
        models: new_list,
    });
    let flow = state.login_flow.as_ref().unwrap();
    assert!(!flow.selected_models.contains(&first));
    assert!(flow.selected_models.contains("brand-new-model"));
}

// S6: user saves before fetch returns — fetch result is ignored
#[test]
fn s6_save_before_fetch_then_fetch_is_ignored() {
    let mut state = drive_to_model_select("minimax");
    state.update(Event::LoginFlowSave);
    assert!(state.open_dialog.is_none());
    assert!(state.login_flow.is_none());
    // Late fetch event arrives; nothing to update.
    state.update(Event::LoginFlowModelsFetched {
        provider: "minimax".into(),
        key: "sk-test".into(),
        models: vec!["late".into()],
    });
    // Still no flow.
    assert!(state.login_flow.is_none());
    // No transient warning from the late fetch.
    assert!(state.transient_message.is_none());
}

// S7: user cancels before fetch returns — fetch result is ignored
#[test]
fn s7_cancel_before_fetch_then_fetch_is_ignored() {
    let mut state = drive_to_model_select("minimax");
    state.update(Event::LoginFlowCancel);
    assert!(state.login_flow.is_none());
    state.update(Event::LoginFlowValidationFailed {
        provider: "minimax".into(),
        key: "sk-test".into(),
        error: "late".into(),
    });
    assert!(state.login_flow.is_none());
    assert!(
        state.transient_message.is_none(),
        "late failure must not surface"
    );
}

// S8: fetch returns empty list — model selector is empty
#[test]
fn s8_empty_fetch_replaces_with_empty_list() {
    let mut state = drive_to_model_select("minimax");
    state.update(Event::LoginFlowModelsFetched {
        provider: "minimax".into(),
        key: "sk-test".into(),
        models: vec![],
    });
    let flow = state.login_flow.as_ref().unwrap();
    assert!(flow.available_models.is_empty());
    assert!(flow.selected_models.is_empty());
}

// S9: unknown provider — no defaults
#[test]
fn s9_unknown_provider_no_defaults() {
    let mut state = AppState::default();
    state.update(Event::LoginFlowStart);
    state.update(Event::LoginFlowSelectProvider {
        provider: "ghost".into(),
    });
    state.update(Event::LoginFlowSubmitKey {
        provider: "ghost".into(),
        key: "k".into(),
    });
    let flow = state.login_flow.as_ref().unwrap();
    assert_eq!(flow.step, LoginStep::ModelSelect);
    assert!(flow.available_models.is_empty());
}

// S10: fetch returns subset of defaults — model is dropped
#[test]
fn s10_fetch_subset_drops_missing_model() {
    let mut state = drive_to_model_select("minimax");
    let original = state.login_flow.as_ref().unwrap().available_models.clone();
    assert!(original.len() >= 2);
    let subset: Vec<String> = original.iter().take(1).cloned().collect();
    state.update(Event::LoginFlowModelsFetched {
        provider: "minimax".into(),
        key: "sk-test".into(),
        models: subset.clone(),
    });
    let flow = state.login_flow.as_ref().unwrap();
    assert_eq!(flow.available_models, subset);
    assert!(flow.selected_models.contains(&subset[0]));
}

// S13: empty key submitted — fetch is not spawned by the main loop,
// but the state still transitions to ModelSelect with defaults.
#[test]
fn s13_empty_key_still_shows_defaults() {
    let mut state = AppState::default();
    state.update(Event::LoginFlowStart);
    state.update(Event::LoginFlowSelectProvider {
        provider: "minimax".into(),
    });
    state.update(Event::LoginFlowSubmitKey {
        provider: "minimax".into(),
        key: "".into(),
    });
    let flow = state.login_flow.as_ref().unwrap();
    assert_eq!(flow.step, LoginStep::ModelSelect);
    assert!(!flow.available_models.is_empty());
}

// -----------------------------------------------------------------------
// Layer 2: open / cancel
// -----------------------------------------------------------------------

#[test]
fn login_command_opens_provider_picker() {
    let mut state = AppState::default();
    state.update(Event::LoginFlowStart);
    assert!(state.open_dialog.is_some());
    assert!(state.login_flow.is_some());
    assert_eq!(
        state.login_flow.as_ref().unwrap().step,
        LoginStep::ProviderPicker
    );
}

#[test]
fn login_select_provider_pushes_key_input() {
    let mut state = AppState::default();
    state.update(Event::LoginFlowStart);
    state.update(Event::LoginFlowSelectProvider {
        provider: "minimax".into(),
    });
    assert_eq!(state.login_flow.as_ref().unwrap().step, LoginStep::KeyInput);
    assert_eq!(state.login_flow.as_ref().unwrap().provider, "minimax");
}

#[test]
fn login_submit_key_preserves_provider_when_empty() {
    let mut state = AppState::default();
    state.update(Event::LoginFlowStart);
    state.update(Event::LoginFlowSelectProvider {
        provider: "minimax".into(),
    });
    state.update(Event::LoginFlowSubmitKey {
        provider: "".into(),
        key: "sk-test".into(),
    });
    let flow = state.login_flow.as_ref().unwrap();
    assert_eq!(flow.step, LoginStep::ModelSelect);
    assert_eq!(flow.provider, "minimax");
    assert_eq!(flow.key, "sk-test");
}

#[test]
fn login_toggle_model_updates_selection() {
    let mut state = drive_to_model_select("minimax");
    let first = state.login_flow.as_ref().unwrap().available_models[0].clone();
    assert!(state
        .login_flow
        .as_ref()
        .unwrap()
        .selected_models
        .contains(&first));
    state.update(Event::LoginFlowToggleModel {
        model: first.clone(),
    });
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
    state.update(Event::LoginFlowStart);
    state.update(Event::LoginFlowCancel);
    assert!(state.open_dialog.is_none());
    assert!(state.login_flow.is_none());
}
