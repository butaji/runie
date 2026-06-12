//! Model commands using the new DSL

use crate::commands::{CommandCategory, CommandRegistry, CommandResult, DialogType};
use crate::dialog::{ItemAction, Panel, PanelStack};
use crate::model::AppState;

pub fn register(registry: &mut CommandRegistry) {
    registry.register(
        crate::cmd!("model")
            .desc("Switch model")
            .aliases(&["m"])
            .category(CommandCategory::Model)
            .handler(handle_model),
    );

    registry.register(
        crate::cmd!("thinking")
            .desc("Set thinking level (off/low/medium/high)")
            .category(CommandCategory::Model)
            .handler(handle_thinking),
    );

    registry.register(
        crate::cmd!("scoped-models")
            .desc("Enable/disable models for cycling")
            .category(CommandCategory::Model)
            .handler(handle_scoped_models),
    );
}

fn handle_model(state: &mut AppState, args: &str) -> CommandResult {
    let rest = args.trim();
    if rest.is_empty() {
        return CommandResult::OpenDialog(DialogType::ModelSelector);
    }
    let parts: Vec<_> = rest.split('/').filter(|s| !s.is_empty()).collect();
    match parts.len() {
        2 => {
            state.config.current_provider = parts[0].to_string();
            state.config.current_model = parts[1].to_string();
        }
        1 => {
            state.config.current_model = parts[0].to_string();
        }
        _ => {
            return CommandResult::Message(format!(
                "Current: {}/{}. Usage: /model provider/model or /model model",
                state.config.current_provider, state.config.current_model
            ));
        }
    }
    CommandResult::Message(format!(
        "Switched to {}/{}",
        state.config.current_provider, state.config.current_model
    ))
}

fn handle_thinking(state: &mut AppState, args: &str) -> CommandResult {
    let rest = args.trim();
    if rest.is_empty() {
        return open_thinking_panel(state);
    }
    match rest.parse::<crate::model::ThinkingLevel>() {
        Ok(level) => {
            state.config.thinking_level = level;
            CommandResult::Message(format!(
                "Thinking level set to: {}",
                state.config.thinking_level.as_str()
            ))
        }
        Err(e) => CommandResult::Message(format!("Error: {e}")),
    }
}

fn open_thinking_panel(state: &mut AppState) -> CommandResult {
    use crate::model::ThinkingLevel;
    let current = state.config.thinking_level;

    let mut panel = Panel::new("thinking", "Thinking Level")
        .header("Select thinking level")
        .header("Tip: /thinking off|low|medium|high also works");

    for &level in ThinkingLevel::all() {
        let label = if level == current {
            format!("{} (current)", level.as_str())
        } else {
            level.as_str().to_string()
        };
        let evt = crate::Event::RunThinkingCommand { level };
        panel = panel.item(&label, ItemAction::Emit(evt));
    }

    CommandResult::OpenPanelStack(PanelStack::new(panel))
}

fn handle_scoped_models(state: &mut AppState, _: &str) -> CommandResult {
    if state.config.scoped_models.is_empty() {
        return CommandResult::Message(
            "No scoped models configured. Add [models.scoped] to config.toml.".into(),
        );
    }
    CommandResult::OpenDialog(DialogType::ScopedModels)
}

// ============================================================================
// Form-submit handlers (called from update/mod.rs with form values).
// ============================================================================

/// Set the thinking level (e.g. from the thinking panel selection).
pub(crate) fn run_thinking(state: &mut AppState, level: crate::model::ThinkingLevel) {
    state.config.thinking_level = level;
    state.add_system_msg(format!(
        "Thinking level set to: {}",
        state.config.thinking_level.as_str()
    ));
}
