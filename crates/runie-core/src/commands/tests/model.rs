//! /model command tests — the selector must only offer models from connected
//! providers and the provider's chosen model list.

use crate::commands::{CommandResult, DialogType};
use crate::model::AppState;
use crate::update::dialog::process_command_result;

fn configure(providers: &[(String, Vec<String>)]) {
    crate::login_config::set_test_config_with_providers(providers);
}

fn reset_config() {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    crate::login_config::set_test_config_path(PathBuf::from(format!(
        "/tmp/runie_model_test_reset_{}.toml",
        n
    )));
}

#[test]
fn model_no_configured_providers_shows_message() {
    reset_config();
    let mut state = AppState::default();
    let result = crate::commands::dsl::handlers::model::handle_model(&mut state, "");
    assert!(
        matches!(result, CommandResult::Message(ref msg) if msg.contains("No connected providers")),
        "expected message about no connected providers, got {:?}",
        result
    );
}

#[test]
fn model_unknown_provider_model_returns_warning() {
    configure(&[("openai".into(), vec!["gpt-4o".into()])]);
    let mut state = AppState::default();
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
    configure(&[("openai".into(), vec!["gpt-4o".into(), "gpt-4o-mini".into()])]);
    let mut state = AppState::default();
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
    configure(&[("openai".into(), vec!["gpt-4o".into()])]);
    let mut state = AppState::default();
    let result = crate::commands::dsl::handlers::model::handle_model(&mut state, "");
    assert!(
        matches!(result, CommandResult::OpenDialog(DialogType::ModelSelector)),
        "expected ModelSelector dialog, got {:?}",
        result
    );

    process_command_result(&mut state, result);

    let items = match &state.open_dialog {
        Some(crate::commands::DialogState::ModelSelector(stack)) => {
            stack.current().map(|p| p.items.clone()).unwrap_or_default()
        }
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
    configure(&[("openai".into(), vec!["gpt-4o".into()])]);
    let mut state = AppState::default();
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
    configure(&[("openai".into(), vec!["gpt-4o".into()])]);
    let mut state = AppState::default();
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
    // Provider returns a model name that is not in the static catalog.
    configure(&[(
        "custom-provider".into(),
        vec!["custom-model".into(), "other-model".into()],
    )]);
    let mut state = AppState::default();
    state.config.current_provider = "custom-provider".into();
    state.config.current_model = "custom-model".into();

    let result = crate::commands::dsl::handlers::model::handle_model(&mut state, "");
    process_command_result(&mut state, result);

    let items = match &state.open_dialog {
        Some(crate::commands::DialogState::ModelSelector(stack)) => {
            stack.current().map(|p| p.items.clone()).unwrap_or_default()
        }
        other => panic!("expected ModelSelector dialog, got {:?}", other),
    };

    let labels: Vec<_> = items.iter().filter_map(|i| i.label()).collect();
    assert!(
        labels.iter().any(|l| l.contains("custom-provider/custom-model")),
        "selector should contain unknown configured model, got: {:?}",
        labels
    );
    assert!(
        labels.iter().any(|l| l.contains("custom-provider/other-model")),
        "selector should contain every configured model, got: {:?}",
        labels
    );
}
