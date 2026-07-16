//! Shared test helpers for AppState manipulation.
//!
//! These helpers are implemented here (not re-exported from `runie_core::tests_support`)
//! so that `runie-testing` does not depend on `runie_core::tests_support` being available
//! at compile time.  `runie-core` owns the canonical implementations in
//! `runie-core/src/tests/support.rs`; `runie-testing` duplicates them here for
//! self-contained test use.

use runie_core::config::ModelProvider;
use runie_core::event::Event;
use runie_core::model::AppState;

/// Seed `state.config.model_providers` with the given provider configurations.
/// Note: api_key is no longer stored in config - it's resolved from keyring/env.
fn seed_providers(state: &mut AppState, providers: &[(String, String, String, Vec<String>)]) {
    for (name, base_url, _api_key, models) in providers {
        state.config_mut().model_providers_mut().insert(
            name.clone(),
            ModelProvider {
                provider_type: None,
                base_url: base_url.clone(),
                models: models.clone(),
<<<<<<< HEAD
                headers: Default::default(),
                context_window_fallbacks: Default::default(),
            },
        );
    }
}

/// Returns a fresh `AppState` with default values and mock provider configured.
pub fn fresh_state() -> AppState {
    let mut state = AppState::default();
    seed_providers(
        &mut state,
        &[("mock".into(), "".into(), "".into(), vec!["echo".into()])],
    );
    state
}

/// Simulates typing `text` into the input buffer of `state`.
pub fn type_str(state: &mut AppState, text: &str) {
    for c in text.chars() {
        state.update(Event::Input(c));
    }
}

/// Set input buffer directly and submit — bypasses the command palette.
/// Use for slash commands that need arguments.
pub fn exec(state: &mut AppState, text: &str) {
    state.input_mut().input = text.into();
    state.input_mut().cursor_pos = text.len();
    state.update(Event::Submit);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_type_str_appends() {
        let mut state = fresh_state();
        assert_eq!(state.input().input, "");
        type_str(&mut state, "hello");
        assert_eq!(state.input().input, "hello");
        type_str(&mut state, " world");
        assert_eq!(state.input().input, "hello world");
    }

    #[test]
    fn shared_exec_submits_command() {
        let mut state = fresh_state();
        assert_eq!(state.input().input, "");
        exec(&mut state, "/save");
    }
}
