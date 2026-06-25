//! Shared test helpers for runie-core tests.
//!
//! Canonical source for `fresh_state()`, `type_str()`, and `exec()` within
//! `runie-core`.  The remaining helpers (`ENV_LOCK`, `tmp_store`,
//! `minimal_session`) also live here because they need access to
//! `runie-core` internals.

use std::sync::Mutex;

use crate::event::Event;
use crate::model::AppState;
use crate::session::store::SessionStore;
use crate::session::Session;

/// Global lock to serialize tests that touch environment variables.
pub static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Returns a fresh `AppState` with default values and config cache populated
/// from the current test config (set by `set_test_config_with_providers`).
pub fn fresh_state() -> AppState {
    let mut state = AppState::default();
    state.populate_cache_from_login_config();
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

/// Creates a temporary session store in the system temp directory.
pub fn tmp_store() -> SessionStore {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dir =
        std::env::temp_dir()
            .join(format!("runie_slash_test_{}_{}", std::process::id(), n));
    let _ = std::fs::remove_dir_all(&dir);
    SessionStore::new(dir)
}

/// Creates a minimal session for testing.
pub fn minimal_session(name: &str) -> Session {
    Session {
        name: name.to_string(),
        created_at: 1.0,
        updated_at: 1.0,
        messages: vec![],
        provider: "mock".into(),
        model: "echo".into(),
        theme_name: "runie".into(),
        thinking_level: crate::model::ThinkingLevel::Off,
        read_only: false,
        display_name: None,
        session_tree: None,
    }
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
    fn shared_exec_sets_input_and_submits() {
        let mut state = fresh_state();
        assert_eq!(state.input().input, "");
        exec(&mut state, "/save");
        assert!(state.input().cursor_pos >= 0);
    }

    #[test]
    fn shared_tmp_store_is_unique() {
        let store1 = tmp_store();
        let store2 = tmp_store();
        assert_ne!(store1.dir(), store2.dir());
    }

    #[test]
    fn shared_minimal_session_has_defaults() {
        let session = minimal_session("test");
        assert_eq!(session.name, "test");
        assert_eq!(session.provider, "mock");
        assert_eq!(session.model, "echo");
        assert_eq!(session.theme_name, "runie");
        assert_eq!(session.thinking_level, crate::model::ThinkingLevel::Off);
        assert!(!session.read_only);
        assert!(session.messages.is_empty());
    }
}
