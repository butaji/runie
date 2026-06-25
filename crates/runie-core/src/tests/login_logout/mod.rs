//! Tests for the `/providers` command (unified provider management).
//!
//! The `/providers` dialog is the primary interface for managing providers.
//! It replaces the old `/login` and `/logout` commands.
//!
//! Flow: /providers → Add → Login flow → Save → Providers dialog → Select model

use crate::Event;

pub(super) mod helpers;
pub(super) use helpers::{
    assert_panel_id, assert_step, assert_transient_contains, current_panel, current_panel_id,
    fetch_models, fetch_models_for, save_login_flow, select_provider, start_login_flow, submit_key,
};

pub(super) fn clean_config() {
    let dir = std::env::temp_dir().join(format!(
        "runie_login_test_{:?}",
        std::thread::current().id()
    ));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("config.toml");
    let _ = std::fs::remove_file(&path);
    crate::login_config::set_test_config_path(path);
}

pub(super) fn default_models_for_provider(provider: &str) -> Vec<String> {
    crate::provider::find_provider(provider)
        .map(|p| p.models.iter().map(|m| m.name.to_string()).collect())
        .unwrap_or_default()
}

pub(super) fn validate_provider(state: &mut crate::model::AppState, provider: &str, key: &str) {
    let models = default_models_for_provider(provider);
    fetch_models_for(state, provider, key, &models);
}

pub(super) fn add_minimax_provider(state: &mut crate::model::AppState) {
    state.update(Event::ProvidersDialog);
    state.update(Event::ProvidersAdd);
    select_provider(state, "minimax");
    submit_key(state, "sk-test");
    validate_provider(state, "minimax", "sk-test");
    save_login_flow(state);
}

pub(super) fn select_minimax_model(state: &mut crate::model::AppState) {
    state.update(Event::ProvidersSelectModel {
        provider: "minimax".into(),
        model: "MiniMax-M3".into(),
    });
}

pub(super) fn add_provider_and_select_model(
    state: &mut crate::model::AppState,
    provider: &str,
    key: &str,
    model: &str,
) {
    state.update(Event::ProvidersDialog);
    state.update(Event::ProvidersAdd);
    select_provider(state, provider);
    submit_key(state, key);
    validate_provider(state, provider, key);
    save_login_flow(state);
    state.update(Event::ProvidersSelectModel {
        provider: provider.into(),
        model: model.into(),
    });
}

mod add_provider;
mod cancel_nav;
mod close_guard;
mod core;
mod disconnect;
mod happy_path;
mod model_select;
mod validation_retry;
