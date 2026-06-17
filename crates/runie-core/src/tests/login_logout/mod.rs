//! Tests for the `/providers` command (unified provider management).
//!
//! The `/providers` dialog is the primary interface for managing providers.
//! It replaces the old `/login` and `/logout` commands.
//!
//! Flow: /providers → Add → Login flow → Save → Providers dialog → Select model

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
    crate::provider_registry::find_provider(provider)
        .map(|p| p.models.iter().map(|m| m.name.to_string()).collect())
        .unwrap_or_default()
}

pub(super) fn validate_provider(state: &mut crate::model::AppState, provider: &str, key: &str) {
    let models = default_models_for_provider(provider);
    state.update(crate::event::LoginFlowEvent::ModelsFetched {
        provider: provider.into(),
        key: key.into(),
        models,
    });
}

pub(super) fn add_minimax_provider(state: &mut crate::model::AppState) {
    state.update(crate::event::DialogEvent::ProvidersDialog);
    state.update(crate::event::DialogEvent::ProvidersAdd);
    state.update(crate::event::LoginFlowEvent::SelectProvider {
        provider: "minimax".into(),
    });
    state.update(crate::event::LoginFlowEvent::SubmitKey {
        provider: "minimax".into(),
        key: "sk-test".into(),
    });
    validate_provider(state, "minimax", "sk-test");
    state.update(crate::event::LoginFlowEvent::Save);
}

pub(super) fn select_minimax_model(state: &mut crate::model::AppState) {
    state.update(crate::event::DialogEvent::ProvidersSelectModel {
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
    state.update(crate::event::DialogEvent::ProvidersDialog);
    state.update(crate::event::DialogEvent::ProvidersAdd);
    state.update(crate::event::LoginFlowEvent::SelectProvider {
        provider: provider.into(),
    });
    state.update(crate::event::LoginFlowEvent::SubmitKey {
        provider: provider.into(),
        key: key.into(),
    });
    validate_provider(state, provider, key);
    state.update(crate::event::LoginFlowEvent::Save);
    state.update(crate::event::DialogEvent::ProvidersSelectModel {
        provider: provider.into(),
        model: model.into(),
    });
}

mod add_provider;
mod close_guard;
mod core;
mod disconnect;
mod edge_cases;
mod login_flow;
mod model_select;
mod multiple;
mod state_machine;
