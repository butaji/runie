#![allow(clippy::all)]
//! Tests for model selection in the login flow and provider dialog.

use crate::login_flow::LoginStep;
use crate::model::AppState;
use crate::Event;

use super::{
    add_minimax_provider, assert_step, assert_transient_contains, clean_config, current_panel,
    fetch_models, save_login_flow, select_minimax_model, select_provider, submit_key,
};

// ---------------------------------------------------------------------------
// Model selection tests
// ---------------------------------------------------------------------------

#[test]
fn providers_select_model_switches_active_model() {
    clean_config();
    let mut state = AppState::default();

    add_minimax_provider(&mut state);
    select_minimax_model(&mut state);

    assert_eq!(state.config.current_provider, "minimax");
    assert_eq!(state.config.current_model, "MiniMax-M3");
}

#[test]
fn providers_select_model_closes_dialog() {
    clean_config();
    let mut state = AppState::default();

    add_minimax_provider(&mut state);
    select_minimax_model(&mut state);

    assert!(
        state.open_dialog.is_none(),
        "selecting a model should close the dialog"
    );
}

#[test]
fn providers_select_model_records_usage() {
    clean_config();
    let mut state = AppState::default();

    add_minimax_provider(&mut state);
    select_minimax_model(&mut state);

    assert!(
        state
            .config
            .recent_models
            .iter()
            .any(|m| m.contains("minimax")),
        "model usage should be recorded in recent_models"
    );
}

// ---------------------------------------------------------------------------
// Edge cases for model selection
// ---------------------------------------------------------------------------

fn start_minimax_flow(state: &mut AppState) {
    state.config.current_provider.clear();
    state.config.current_model.clear();
    state.update(crate::Event::ProvidersDialog);
    state.update(crate::Event::ProvidersAdd);
    select_provider(state, "minimax");
    submit_key(state, "sk-test");
}

#[test]
fn empty_model_list_reaches_model_select_but_save_rejected() {
    clean_config();
    let mut state = AppState::default();
    start_minimax_flow(&mut state);
    fetch_models(&mut state, &[]);

    assert_step(&state, LoginStep::ModelSelect);

    save_login_flow(&mut state);

    assert!(
        state.login_flow.is_some(),
        "save should be rejected with no models selected"
    );
    assert_transient_contains(&state, "Select at least one model");
}

#[test]
fn deselect_all_models_rejects_save() {
    clean_config();
    let mut state = AppState::default();
    start_minimax_flow(&mut state);
    fetch_models(&mut state, &["M3".into(), "M2".into()]);

    state.update(crate::Event::ToggleModel { model: "M3".into() });
    state.update(crate::Event::ToggleModel { model: "M2".into() });
    save_login_flow(&mut state);

    assert!(
        state.login_flow.is_some(),
        "save should be rejected when all models are deselected"
    );
    assert_transient_contains(&state, "Select at least one model");
}

#[test]
fn single_model_enter_saves() {
    clean_config();
    let mut state = AppState::default();
    start_minimax_flow(&mut state);
    fetch_models(&mut state, &["M3".into()]);

    state.update(Event::from(crate::Event::Submit));

    assert!(
        state.login_flow.is_none(),
        "login flow should be cleared after save"
    );
    assert_eq!(state.config.current_provider, "minimax");
    assert_eq!(state.config.current_model, "M3");
    assert!(state.has_models());
}

#[test]
fn multiple_models_toggle_first_save_activates_second() {
    clean_config();
    let mut state = AppState::default();
    start_minimax_flow(&mut state);
    fetch_models(&mut state, &["M3".into(), "M2".into()]);

    state.update(crate::Event::ToggleModel { model: "M3".into() });
    save_login_flow(&mut state);

    assert!(state.login_flow.is_none());
    assert_eq!(state.config.current_provider, "minimax");
    assert_eq!(state.config.current_model, "M2");
}

#[test]
fn toggle_unchecked_model_then_save() {
    clean_config();
    let mut state = AppState::default();
    start_minimax_flow(&mut state);
    fetch_models(&mut state, &["M3".into(), "M2".into()]);

    state.update(crate::Event::ToggleModel { model: "M3".into() });
    state.update(crate::Event::ToggleModel { model: "M3".into() });
    save_login_flow(&mut state);

    assert!(state.login_flow.is_none());
    assert_eq!(state.config.current_provider, "minimax");
    assert_eq!(state.config.current_model, "M3");
}

#[test]
fn space_toggles_model_checkbox() {
    clean_config();
    let mut state = AppState::default();
    start_minimax_flow(&mut state);
    fetch_models(&mut state, &["M3".into(), "M2".into()]);

    state.update(Event::from(crate::Event::Input(' ')));

    let flow = state.login_flow.as_ref().expect("flow still open");
    assert!(
        !flow.selected_models.contains("M3"),
        "space should toggle off the first selected model"
    );
    assert!(
        state.open_dialog.is_some(),
        "dialog should remain open after toggling"
    );
}

#[test]
fn toggle_model_event_preserves_selection_index() {
    clean_config();
    let mut state = AppState::default();
    start_minimax_flow(&mut state);
    fetch_models(&mut state, &["M3".into(), "M2".into(), "M1".into()]);

    state.update(Event::from(crate::Event::HistoryNext));
    assert_eq!(
        current_panel(&state).map(|p| p.selected),
        Some(1),
        "selection should start on second model"
    );

    state.update(Event::from(crate::Event::ToggleModel {
        model: "M2".into(),
    }));

    assert_eq!(
        current_panel(&state).map(|p| p.selected),
        Some(1),
        "toggle event should keep selection on the toggled item"
    );
}

#[test]
fn space_toggle_preserves_selection_index() {
    clean_config();
    let mut state = AppState::default();
    start_minimax_flow(&mut state);
    fetch_models(&mut state, &["M3".into(), "M2".into(), "M1".into()]);

    state.update(Event::from(crate::Event::HistoryNext));
    assert_eq!(
        current_panel(&state).map(|p| p.selected),
        Some(1),
        "selection should start on second model"
    );

    state.update(Event::from(crate::Event::Input(' ')));

    assert_eq!(
        current_panel(&state).map(|p| p.selected),
        Some(1),
        "space toggle should keep selection on the toggled item"
    );
    let flow = state.login_flow.as_ref().expect("flow still open");
    assert!(
        !flow.selected_models.contains("M2"),
        "space should toggle off the second selected model"
    );
}

#[test]
fn saving_after_deselecting_model_persists_two_models() {
    clean_config();
    let mut state = AppState::default();
    start_minimax_flow(&mut state);
    fetch_models(&mut state, &["M3".into(), "M2".into(), "M1".into()]);

    state.update(crate::Event::ToggleModel { model: "M1".into() });
    save_login_flow(&mut state);

    let configured = crate::provider::config::list_configured_providers();
    let (_, _, models) = configured
        .into_iter()
        .find(|(p, _, _)| p == "minimax")
        .expect("minimax should be configured");
    assert_eq!(
        models.len(),
        2,
        "only two still-selected models should be saved"
    );
    assert!(!models.contains(&"M1".into()));
}
