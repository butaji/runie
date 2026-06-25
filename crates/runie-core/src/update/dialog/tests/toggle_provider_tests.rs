//! Tests for provider model toggle functionality.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::config::Config;
use crate::model::AppState;
use crate::update::dialog::toggles::{parse_provider_model_toggle, toggle_provider_model};

fn temp_config_path() -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    PathBuf::from(format!(
        "/tmp/runie_provider_toggle_test_{}_{}.toml",
        std::process::id(),
        n
    ))
}

#[test]
fn parse_settings_toggle_key_extracts_provider_and_model() {
    assert_eq!(
        parse_provider_model_toggle("edit_provider:openai:gpt-4o"),
        Some(("openai", "gpt-4o"))
    );
}

#[test]
fn parse_settings_toggle_key_rejects_malformed_keys() {
    assert!(parse_provider_model_toggle("edit_provider:gpt-4o").is_none());
    assert!(parse_provider_model_toggle("other:openai:gpt-4o").is_none());
}

#[test]
fn toggle_provider_model_disables_model_and_switches_active() {
    let path = temp_config_path();
    crate::login_config::set_test_config_path(path);
    crate::login_config::save_provider_config(
        "openai",
        "https://api.openai.com/v1",
        "sk-test",
        &["gpt-4o".into(), "gpt-4o-mini".into()],
    )
    .unwrap();

    let mut state = AppState::default();
    state.config_mut().current_provider = "openai".into();
    state.config_mut().current_model = "gpt-4o-mini".into();
    // Initialize the config cache so toggle_provider_model can update it
    state.config_cache = Some(Config::default());

    toggle_provider_model(&mut state, "openai", "gpt-4o-mini");

    // Verify cache was updated (synchronous update)
    let cached_models = state
        .config_cache
        .as_ref()
        .and_then(|c| c.model_providers.get("openai"))
        .map(|p| p.models.clone())
        .unwrap_or_default();
    assert_eq!(cached_models, vec!["gpt-4o"]);
    assert_eq!(state.config().current_model, "gpt-4o");
}

#[test]
fn toggle_provider_model_enables_missing_model() {
    let path = temp_config_path();
    crate::login_config::set_test_config_path(path);
    crate::login_config::save_provider_config(
        "openai",
        "https://api.openai.com/v1",
        "sk-test",
        &["gpt-4o".into()],
    )
    .unwrap();

    let mut state = AppState::default();
    state.config_mut().current_provider = "openai".into();
    state.config_mut().current_model = "gpt-4o".into();
    // Initialize the config cache so toggle_provider_model can update it
    state.config_cache = Some(Config::default());

    toggle_provider_model(&mut state, "openai", "gpt-4o-mini");

    // Verify cache was updated (synchronous update)
    let cached_models = state
        .config_cache
        .as_ref()
        .and_then(|c| c.model_providers.get("openai"))
        .map(|p| p.models.clone())
        .unwrap_or_default();
    assert!(cached_models.contains(&"gpt-4o".to_string()));
    assert!(cached_models.contains(&"gpt-4o-mini".to_string()));
}
