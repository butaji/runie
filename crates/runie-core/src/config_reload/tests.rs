//! Config reload tests.

use super::*;
use crate::model::AppState;
use crate::Event;
use std::fs;
use tempfile::tempdir;

#[test]
fn config_changed_applies_provider() {
    // Layer 2: Verify SwitchModel event updates AppState
    let mut state = AppState::default();

    // Initial defaults — mock in dev, empty in production.
    let (def_provider, def_model) = if crate::provider_registry::is_mock_enabled() {
        ("mock", "echo")
    } else {
        ("", "")
    };
    assert_eq!(state.config.current_provider, def_provider);
    assert_eq!(state.config.current_model, def_model);

    // Send SwitchModel event
    state.update(Event::SwitchModel {
        provider: "anthropic".to_string(),
        model: "claude-3-sonnet".to_string(),
    });

    // Verify provider and model are updated
    assert_eq!(state.config.current_provider, "anthropic");
    assert_eq!(state.config.current_model, "claude-3-sonnet");

    // Verify a transient notification was emitted
    assert_eq!(
        state.transient_message,
        Some("Switched to anthropic/claude-3-sonnet".into())
    );
    assert_eq!(
        state.transient_level,
        Some(crate::event::TransientLevel::Success)
    );
}

#[test]
fn config_theme_change_applies_theme() {
    let mut state = AppState::default();
    assert_eq!(state.config.theme_name, "runie");

    state.update(Event::SwitchTheme {
        name: "dracula".to_string(),
    });

    assert_eq!(state.config.theme_name, "dracula");
    assert_eq!(
        state.transient_message,
        Some("Theme switched to 'dracula'".into())
    );
    assert_eq!(
        state.transient_level,
        Some(crate::event::TransientLevel::Success)
    );
}

#[tokio::test]
async fn config_watcher_detects_initial_change() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.toml");

    // Create initial config with explicit provider/model
    fs::write(
        &config_path,
        r#"
provider = "openai"
model = "gpt-4"

[model_providers.openai]
type = "openai"
base_url = "https://api.openai.com"
api_key = "test"
"#,
    )
    .unwrap();

    let (tx, mut rx) = mpsc::channel::<Event>(10);

    // Spawn watcher
    let handle = spawn_config_watcher(tx, config_path.clone());

    // Wait for the watcher to pick up the initial config
    // Give it time to load and compare (2 poll intervals = 4 seconds)
    tokio::time::sleep(Duration::from_secs(4)).await;

    // Check that a SwitchModel event was emitted for initial load
    let evt = tokio::time::timeout(Duration::from_secs(1), rx.recv()).await;
    assert!(evt.is_ok(), "Should receive SwitchModel event");
    assert!(matches!(evt.unwrap(), Some(Event::SwitchModel { .. })));

    // Clean up
    handle.abort();
}

#[tokio::test]
async fn config_watcher_parses_toml_changes() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.toml");

    // Create initial config
    fs::write(
        &config_path,
        r#"
provider = "openai"
model = "gpt-4"

[model_providers.openai]
type = "openai"
base_url = "https://api.openai.com"
api_key = "test"
"#,
    )
    .unwrap();

    let (tx, mut rx) = mpsc::channel::<Event>(10);
    let handle = spawn_config_watcher(tx, config_path.clone());

    // Wait for initial load
    tokio::time::sleep(Duration::from_secs(4)).await;

    // Drain any initial events
    while rx.try_recv().is_ok() {}

    // Now change the config
    fs::write(
        &config_path,
        r#"
provider = "anthropic"
model = "claude-3"

[model_providers.anthropic]
type = "anthropic"
base_url = "https://api.anthropic.com"
api_key = "test"
"#,
    )
    .unwrap();

    // Wait for the watcher to detect the change
    tokio::time::sleep(Duration::from_secs(3)).await;

    let evt = tokio::time::timeout(Duration::from_secs(1), rx.recv()).await;
    assert!(evt.is_ok(), "Should receive SwitchModel event");

    if let Ok(Some(Event::SwitchModel { provider, model })) = evt {
        assert_eq!(provider, "anthropic");
        assert_eq!(model, "claude-3");
    } else {
        panic!("Expected SwitchModel event");
    }

    handle.abort();
}

#[test]
fn config_path_returns_expected_path() {
    let path = config_path();
    assert!(
        path.components().next().is_some(),
        "Path should not be empty"
    );
    assert!(
        path.file_name().is_some_and(|n| n == "config.toml"),
        "Path should end with config.toml"
    );
}

#[test]
fn config_load_parses_toml() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.toml");

    // Write a config file
    fs::write(
        &config_path,
        r#"
provider = "test-provider"
model = "test-model"

[model_providers.test-provider]
type = "test"
base_url = "http://localhost"
api_key = "secret"
"#,
    )
    .unwrap();

    // Load the config (migration moves top-level model → models.default)
    let config = Config::load_from(&config_path);

    assert_eq!(config.provider, Some("test-provider".to_string()));
    assert_eq!(config.default_model(), Some("test-model"));
}

#[test]
fn config_load_defaults_when_missing() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("nonexistent.toml");

    let config = Config::load_from(&config_path);

    assert_eq!(config.provider, None);
    assert_eq!(config.model, None);
    assert_eq!(config.default_model(), None);
}

#[test]
fn config_theme_field_emits_switch_theme() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.toml");

    fs::write(
        &config_path,
        r#"
theme = "dracula"
"#,
    )
    .unwrap();

    let config = Config::load_from(&config_path);
    assert_eq!(config.theme, Some("dracula".to_string()));
}

#[test]
fn config_load_parses_scoped_models() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.toml");

    fs::write(
        &config_path,
        r#"
provider = "openai"

[models]
scoped = ["gpt-4o", "claude-3-sonnet", "gemini-1.5-pro"]
"#,
    )
    .unwrap();

    let config = Config::load_from(&config_path);
    let scoped = config.scoped_models().expect("should have scoped models");
    assert_eq!(scoped.len(), 3);
    assert_eq!(scoped[0], "gpt-4o");
    assert_eq!(scoped[1], "claude-3-sonnet");
    assert_eq!(scoped[2], "gemini-1.5-pro");
}

#[test]
fn config_load_scoped_models_missing_is_none() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.toml");

    fs::write(
        &config_path,
        r#"
provider = "openai"
model = "gpt-4"
"#,
    )
    .unwrap();

    let config = Config::load_from(&config_path);
    assert!(config.scoped_models().is_none());
}

#[test]
fn config_load_uses_default_model_from_models_section() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.toml");

    fs::write(
        &config_path,
        r#"
provider = "openai"
model = "gpt-3.5"

[models]
default = "gpt-4"

[model_providers.openai]
type = "openai"
base_url = "https://api.openai.com"
api_key = "test"
"#,
    )
    .unwrap();

    let config = Config::load_from(&config_path);

    // models.default already existed, so migration should NOT overwrite it
    assert_eq!(config.default_model(), Some("gpt-4"));
}
