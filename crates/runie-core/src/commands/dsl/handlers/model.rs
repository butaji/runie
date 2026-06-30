//! Model commands.

use crate::commands::{CommandResult, DialogType};
use crate::dialog::{ItemAction, Panel, PanelStack};
use crate::model::{AppState, ThinkingLevel};
use crate::provider::is_mock_enabled;

/// Returns true if there is at least one usable provider: a configured TOML
/// provider, or the mock provider when `RUNIE_MOCK` is set.
fn has_any_available_provider(state: &AppState) -> bool {
    !state.configured_providers().is_empty() || is_mock_enabled()
}

/// Register all model handlers with the handler registry (for YAML-based commands).
pub fn register_handlers(registry: &mut crate::commands::dsl::handlers::registry::HandlerRegistry) {
    use crate::register_handler;
    register_handler!(registry, "model", Handler(handle_model));
    register_handler!(registry, "thinking", Handler(handle_thinking));
    register_handler!(registry, "scoped-models", Handler(handle_scoped_models));
}

pub fn handle_model(state: &mut AppState, args: &str) -> CommandResult {
    let rest = args.trim();
    if rest.is_empty() {
        return if !has_any_available_provider(state) {
            CommandResult::Message(
                "No connected providers. Use /provider to add a provider first.".into(),
            )
        } else {
            CommandResult::OpenDialog(DialogType::ModelSelector)
        };
    }
    let parts: Vec<_> = rest.split('/').filter(|s| !s.is_empty()).collect();
    match parts.len() {
        2 => switch_to_model(state, parts[0], parts[1]),
        1 => {
            let provider = state.config_mut().current_provider.clone();
            switch_to_model(state, &provider, parts[0])
        }
        _ => CommandResult::Message(format!(
            "Current: {}/{}. Format: /model provider/model or /model model",
            state.config().current_provider,
            state.config().current_model
        )),
    }
}

fn switch_to_model(state: &mut AppState, provider: &str, model: &str) -> CommandResult {
    if !is_model_configured(state, provider, model) {
        return CommandResult::Warning(format!(
            "Model {}/{} is not available. Connect the provider and choose models with /provider.",
            provider, model
        ));
    }
    state.switch_model(provider.to_owned(), model.to_owned(), true);
    CommandResult::Message(format!("Switched to {}/{}", provider, model))
}

fn is_model_configured(state: &AppState, provider: &str, model: &str) -> bool {
    state
        .configured_providers()
        .iter()
        .any(|(p, _, models)| p == provider && models.contains(&model.to_owned()))
}

pub fn handle_thinking(state: &mut AppState, args: &str) -> CommandResult {
    let rest = args.trim();
    if rest.is_empty() {
        return open_thinking_panel(state);
    }
    match rest.parse::<ThinkingLevel>() {
        Ok(level) => {
            state.config_mut().thinking_level = level;
            CommandResult::Message(format!(
                "Thinking level set to: {}",
                state.config().thinking_level.as_str()
            ))
        }
        Err(e) => CommandResult::Message(format!("Error: {e}")),
    }
}

fn open_thinking_panel(state: &mut AppState) -> CommandResult {
    let current = state.config().thinking_level;

    let mut panel = Panel::new("thinking", "Thinking Level")
        .header("Select thinking level")
        .header("Tip: /thinking off|low|medium|high also works");

    for &level in ThinkingLevel::all() {
        let label = if level == current {
            format!("{} (current)", level.as_str())
        } else {
            level.as_str().to_owned()
        };
        let evt = crate::Event::RunThinkingCommand { level };
        panel = panel.item(&label, ItemAction::Emit(evt));
    }

    CommandResult::OpenPanelStack(Box::new(PanelStack::new(panel)))
}

pub fn handle_scoped_models(state: &mut AppState, _: &str) -> CommandResult {
    if !has_any_available_provider(state) {
        return CommandResult::Message(
            "No connected providers. Use /provider to add a provider first.".into(),
        );
    }
    CommandResult::OpenDialog(DialogType::ScopedModels)
}

pub fn run_thinking(state: &mut AppState, level: ThinkingLevel) {
    state.config_mut().thinking_level = level;
    state.add_system_msg(format!(
        "Thinking level set to: {}",
        state.config().thinking_level.as_str()
    ));
}
