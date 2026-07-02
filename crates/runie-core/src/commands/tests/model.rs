//! /model command tests — the selector must only offer models from connected
//! providers and the provider's chosen model list.

use crate::commands::{CommandResult, DialogKind, DialogType};
use crate::config::{Config, ModelProvider};
use crate::model::AppState;
use crate::update::dialog::process_command_result;

/// Build a Config with the given provider/models.
fn make_config(providers: &[(String, Vec<String>)]) -> Config {
    let mut cfg = Config::default();
    for (name, models) in providers {
        cfg.model_providers.insert(
            name.clone(),
            ModelProvider {
                provider_type: None,
                base_url: format!("https://{}.example.com", name),
                models: models.clone(),
            },
        );
    }
    cfg
}

/// Set state.config().model_providers with the given provider/models.
fn set_config(state: &mut AppState, providers: &[(String, Vec<String>)]) {
    let cfg = make_config(providers);
    *state.config_mut().model_providers_mut() = cfg.model_providers;
}

fn reset_config() {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    crate::provider::config::set_test_config_path(PathBuf::from(format!(
        "/tmp/runie_model_test_reset_{}.toml",
        n
    )));
}

#[test]
fn model_no_configured_providers_shows_message() {
    // Unset RUNIE_MOCK env var (dev.sh sets it to "") and clear thread-local override.
    std::env::remove_var("RUNIE_MOCK");
    std::env::remove_var("RUNIE_MOCK_DELAY");
    crate::provider::set_mock_enabled(false);
    reset_config();
    let mut state = AppState::default();
    let result = crate::commands::dsl::handlers::model::handle_model(&mut state, "");
    crate::provider::set_mock_enabled(false); // clear thread-local override
    assert!(
        matches!(result, CommandResult::Message(ref msg) if msg.contains("No connected providers")),
        "expected message about no connected providers, got {:?}",
        result
    );
}

#[test]
fn model_mock_enabled_opens_selector_even_without_toml_config() {
    // When RUNIE_MOCK is set but no TOML providers are configured,
    // /model should still open the selector (showing mock/echo).
    std::env::remove_var("RUNIE_MOCK");
    std::env::remove_var("RUNIE_MOCK_DELAY");
    crate::provider::set_mock_enabled(true);
    reset_config();
    let mut state = AppState::default();
    let result = crate::commands::dsl::handlers::model::handle_model(&mut state, "");
    crate::provider::set_mock_enabled(false); // clear thread-local override
    assert!(
        matches!(result, CommandResult::OpenDialog(DialogType::ModelSelector)),
        "expected ModelSelector dialog with mock enabled, got {:?}",
        result
    );
}

#[test]
fn model_unknown_provider_model_returns_warning() {
    let mut state = AppState::default();
    set_config(&mut state, &[("openai".into(), vec!["gpt-4o".into()])]);
    state.config.current_provider = "openai".into();
    state.config.current_model = "gpt-4o".into();
    let result = crate::commands::dsl::handlers::model::handle_model(
        &mut state,
        "anthropic/claude-3-sonnet",
    );
    assert!(
        matches!(result, CommandResult::Warning(ref msg) if msg.contains("not available")),
        "expected warning for unconfigured model, got {:?}",
        result
    );
}

#[test]
fn model_known_model_switches() {
    let mut state = AppState::default();
    set_config(
        &mut state,
        &[("openai".into(), vec!["gpt-4o".into(), "gpt-4o-mini".into()])],
    );
    state.config.current_provider = "openai".into();
    state.config.current_model = "gpt-4o".into();
    let result = crate::commands::dsl::handlers::model::handle_model(&mut state, "gpt-4o-mini");
    assert!(
        matches!(result, CommandResult::Message(ref msg) if msg.contains("Switched")),
        "expected switch message, got {:?}",
        result
    );
    assert_eq!(state.config.current_model, "gpt-4o-mini");
}

#[test]
fn model_opens_selector_with_only_configured_models() {
    let mut state = AppState::default();
    set_config(&mut state, &[("openai".into(), vec!["gpt-4o".into()])]);
    let result = crate::commands::dsl::handlers::model::handle_model(&mut state, "");
    assert!(
        matches!(result, CommandResult::OpenDialog(DialogType::ModelSelector)),
        "expected ModelSelector dialog, got {:?}",
        result
    );

    process_command_result(&mut state, result);

    let items = match &state.open_dialog {
        Some(crate::commands::DialogState::Active {
            kind: DialogKind::ModelSelector,
            panels: stack,
        }) => stack.current().map(|p| p.items.clone()).unwrap_or_default(),
        other => panic!("expected ModelSelector dialog, got {:?}", other),
    };

    let labels: Vec<_> = items.iter().filter_map(|i| i.label()).collect();
    assert!(
        labels.iter().any(|l| l.contains("openai/gpt-4o")),
        "selector should contain configured model, got: {:?}",
        labels
    );
    assert!(
        !labels.iter().any(|l| l.contains("anthropic")),
        "selector should not contain unconfigured providers, got: {:?}",
        labels
    );
    assert!(
        !labels.iter().any(|l| l.contains("gpt-4o-mini")),
        "selector should not contain unchosen models, got: {:?}",
        labels
    );
}

#[test]
fn model_unknown_model_name_for_configured_provider_returns_warning() {
    let mut state = AppState::default();
    set_config(&mut state, &[("openai".into(), vec!["gpt-4o".into()])]);
    state.config.current_provider = "openai".into();
    state.config.current_model = "gpt-4o".into();

    let result =
        crate::commands::dsl::handlers::model::handle_model(&mut state, "openai/nonexistent-model");
    assert!(
        matches!(result, CommandResult::Warning(ref msg) if msg.contains("not available")),
        "expected warning for unknown model name, got {:?}",
        result
    );
    assert_eq!(state.config.current_model, "gpt-4o");
}

#[test]
fn model_unknown_model_name_without_provider_returns_warning() {
    let mut state = AppState::default();
    set_config(&mut state, &[("openai".into(), vec!["gpt-4o".into()])]);
    state.config.current_provider = "openai".into();
    state.config.current_model = "gpt-4o".into();

    let result =
        crate::commands::dsl::handlers::model::handle_model(&mut state, "nonexistent-model");
    assert!(
        matches!(result, CommandResult::Warning(ref msg) if msg.contains("not available")),
        "expected warning for unknown model name, got {:?}",
        result
    );
    assert_eq!(state.config.current_model, "gpt-4o");
}

#[test]
fn model_selector_includes_unknown_configured_models() {
    let mut state = AppState::default();
    set_config(
        &mut state,
        &[(
            "custom-provider".into(),
            vec!["custom-model".into(), "other-model".into()],
        )],
    );
    state.config.current_provider = "custom-provider".into();
    state.config.current_model = "custom-model".into();

    let result = crate::commands::dsl::handlers::model::handle_model(&mut state, "");
    process_command_result(&mut state, result);

    let items = match &state.open_dialog {
        Some(crate::commands::DialogState::Active {
            kind: DialogKind::ModelSelector,
            panels: stack,
        }) => stack.current().map(|p| p.items.clone()).unwrap_or_default(),
        other => panic!("expected ModelSelector dialog, got {:?}", other),
    };

    let labels: Vec<_> = items.iter().filter_map(|i| i.label()).collect();
    assert!(
        labels
            .iter()
            .any(|l| l.contains("custom-provider/custom-model")),
        "selector should contain unknown configured model, got: {:?}",
        labels
    );
    assert!(
        labels
            .iter()
            .any(|l| l.contains("custom-provider/other-model")),
        "selector should contain every configured model, got: {:?}",
        labels
    );
}
