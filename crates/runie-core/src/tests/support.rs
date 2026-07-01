//! Core-specific test helpers.
//!
//! Contains helpers that need access to `runie-core` internals:
//! - `ENV_LOCK` — global lock for env-sensitive tests
//! - `seed_providers()` — seeds config with provider definitions
//! - `tmp_store()` — creates a temp session store
//! - `minimal_session()` — creates a minimal test session
//!
//! Shared helpers (`fresh_state`, `type_str`, `exec`) are now imported
//! from `runie_testing` instead of duplicated here.

use std::sync::Mutex;

use crate::config::ModelProvider;
use crate::model::AppState;
use crate::session::store::SessionStore;
use crate::session::Session;

/// Global lock to serialize tests that touch environment variables.
pub static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Seed `state.config.model_providers` with the given provider configurations.
/// Each entry is `(Name, base_url, api_key, models)`.
pub fn seed_providers(state: &mut AppState, providers: &[(String, String, String, Vec<String>)]) {
    for (name, base_url, api_key, models) in providers {
        state.config_mut().model_providers_mut().insert(
            name.clone(),
            ModelProvider {
                provider_type: None,
                base_url: base_url.clone(),
                api_key: api_key.clone(),
                models: models.clone(),
            },
        );
    }
}

/// Creates a temporary session store in the system temp directory.
pub fn tmp_store() -> SessionStore {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("runie_slash_test_{}_{}", std::process::id(), n));
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
