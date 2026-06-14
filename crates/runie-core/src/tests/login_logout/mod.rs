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

pub(super) fn add_minimax_provider(state: &mut crate::model::AppState) {
    state.update(crate::event::Event::ProvidersDialog);
    state.update(crate::event::Event::ProvidersAdd);
    state.update(crate::event::Event::LoginFlowSelectProvider {
        provider: "minimax".into(),
    });
    state.update(crate::event::Event::LoginFlowSubmitKey {
        provider: "minimax".into(),
        key: "sk-test".into(),
    });
    state.update(crate::event::Event::LoginFlowSave);
}

pub(super) fn select_minimax_model(state: &mut crate::model::AppState) {
    state.update(crate::event::Event::ProvidersSelectModel {
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
    state.update(crate::event::Event::ProvidersDialog);
    state.update(crate::event::Event::ProvidersAdd);
    state.update(crate::event::Event::LoginFlowSelectProvider {
        provider: provider.into(),
    });
    state.update(crate::event::Event::LoginFlowSubmitKey {
        provider: provider.into(),
        key: key.into(),
    });
    state.update(crate::event::Event::LoginFlowSave);
    state.update(crate::event::Event::ProvidersSelectModel {
        provider: provider.into(),
        model: model.into(),
    });
}

mod add_provider;
mod core;
mod disconnect;
mod edge_cases;
mod login_flow;
mod model_select;
mod multiple;
mod state_machine;
