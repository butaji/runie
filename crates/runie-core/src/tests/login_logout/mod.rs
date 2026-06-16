//! Tests for the `/providers` command (unified provider management).
//!
//! The `/providers` dialog is the primary interface for managing providers.
//! It replaces the old `/login` and `/logout` commands.
//!
//! Flow: /providers → Add → Login flow → Save → Providers dialog → Select model

use crate::event::{InputEvent, ControlEvent, ModelConfigEvent, SystemEvent, DialogEvent, ScrollEvent, AgentEvent, SessionEvent, EditEvent, CommandEvent, DurableCoreEvent, LoginFlowEvent};

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
    state.update(crate::event::Event::Dialog(DialogEvent::ProvidersDialog));
    state.update(crate::event::Event::Dialog(DialogEvent::ProvidersAdd));
    state.update(crate::event::Event::LoginFlow(LoginFlowEvent::SelectProvider {
        provider: "minimax".into(),
    }));
    state.update(crate::event::Event::LoginFlow(LoginFlowEvent::SubmitKey {
        provider: "minimax".into(),
        key: "sk-test".into(),
    }));
    state.update(crate::event::Event::LoginFlow(LoginFlowEvent::Save));
}

pub(super) fn select_minimax_model(state: &mut crate::model::AppState) {
    state.update(crate::event::Event::Dialog(crate::event::DialogEvent::ProvidersSelectModel {
        provider: "minimax".into(),
        model: "MiniMax-M3".into(),
    }));
}

pub(super) fn add_provider_and_select_model(
    state: &mut crate::model::AppState,
    provider: &str,
    key: &str,
    model: &str,
) {
    state.update(crate::event::Event::Dialog(DialogEvent::ProvidersDialog));
    state.update(crate::event::Event::Dialog(DialogEvent::ProvidersAdd));
    state.update(crate::event::Event::LoginFlow(LoginFlowEvent::SelectProvider {
        provider: provider.into(),
    }));
    state.update(crate::event::Event::LoginFlow(LoginFlowEvent::SubmitKey {
        provider: provider.into(),
        key: key.into(),
    }));
    state.update(crate::event::Event::LoginFlow(LoginFlowEvent::Save));
    state.update(crate::event::Event::Dialog(crate::event::DialogEvent::ProvidersSelectModel {
        provider: provider.into(),
        model: model.into(),
    }));
}

mod add_provider;
mod core;
mod disconnect;
mod edge_cases;
mod login_flow;
mod model_select;
mod multiple;
mod state_machine;
