//! Model commands.

use crate::commands::dsl::handlers::NamedHandler;
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
    registry.register("model", NamedHandler::Handler(handle_model));
    registry.register("thinking", NamedHandler::Handler(handle_thinking));
    registry.register("scoped-models", NamedHandler::Handler(handle_scoped_models));
}

pub fn handle_model(state: &mut AppState, args: &str) -> CommandResult {
    use crate::ui_strings::model as m;
    let rest = args.trim();
    if rest.is_empty() {
        return if !has_any_available_provider(state) {
            CommandResult::Message(m::NO_PROVIDERS.into())
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
        _ => CommandResult::Message(m::usage(
            &state.config().current_provider,
            &state.config().current_model,
        )),
    }
}

fn switch_to_model(state: &mut AppState, provider: &str, model: &str) -> CommandResult {
    use crate::ui_strings::model as m;
    if !is_model_configured(state, provider, model) {
        return CommandResult::Warning(m::model_unavailable(provider, model));
    }
    state.switch_model(provider.to_owned(), model.to_owned(), true);
    CommandResult::Message(m::model_switched(provider, model))
}

fn is_model_configured(state: &AppState, provider: &str, model: &str) -> bool {
    state
        .configured_providers()
        .iter()
        .any(|(p, _, models)| p == provider && models.contains(&model.to_owned()))
}

pub fn handle_thinking(state: &mut AppState, args: &str) -> CommandResult {
    use crate::ui_strings::model as m;
    let rest = args.trim();
    if rest.is_empty() {
        return open_thinking_panel(state);
    }
    match rest.parse::<ThinkingLevel>() {
        Ok(level) => {
            state.config_mut().thinking_level = level;
            CommandResult::Message(m::thinking_level(state.config().thinking_level.as_str()))
        }
        Err(e) => CommandResult::Message(m::thinking_error(&e.to_string())),
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
    use crate::ui_strings::model as m;
    if !has_any_available_provider(state) {
        return CommandResult::Message(m::NO_PROVIDERS.into());
    }
    CommandResult::OpenDialog(DialogType::ScopedModels)
}

pub fn run_thinking(state: &mut AppState, level: ThinkingLevel) {
    use crate::ui_strings::model as m;
    state.config_mut().thinking_level = level;
    state.add_system_msg(m::thinking_level(state.config().thinking_level.as_str()));
}
