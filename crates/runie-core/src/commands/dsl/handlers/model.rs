//! Model commands.

use crate::commands::{CommandCategory, CommandRegistry, CommandResult, DialogType};
use crate::dialog::{ItemAction, Panel, PanelStack};
use crate::model::AppState;

use super::spec::{CommandKind, CommandSpec};

static MODEL_COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        name: "model",
        desc: "Switch model",
        aliases: &["m"],
        category: CommandCategory::Model,
        sub: true,
        kind: CommandKind::Handler(handle_model),
    },
    CommandSpec {
        name: "thinking",
        desc: "Set thinking level (off/low/medium/high)",
        aliases: &[],
        category: CommandCategory::Model,
        sub: true,
        kind: CommandKind::Handler(handle_thinking),
    },
    CommandSpec {
        name: "scoped-models",
        desc: "Enable/disable models for cycling",
        aliases: &[],
        category: CommandCategory::Model,
        sub: true,
        kind: CommandKind::Handler(handle_scoped_models),
    },
];

pub fn register(registry: &mut CommandRegistry) {
    super::spec::register_commands(registry, MODEL_COMMANDS);
}

pub fn handle_model(state: &mut AppState, args: &str) -> CommandResult {
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
                "Current: {}/{}. Format: /model provider/model or /model model",
                state.config.current_provider, state.config.current_model
            ));
        }
    }
    state.configure_token_tracker();
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
        let evt = crate::event::CommandEvent::RunThinkingCommand { level };
        panel = panel.item(&label, ItemAction::Emit(evt));
    }

    CommandResult::OpenPanelStack(Box::new(PanelStack::new(panel)))
}

fn handle_scoped_models(state: &mut AppState, _: &str) -> CommandResult {
    if state.config.scoped_models.is_empty() {
        return CommandResult::Message(
            "No scoped models configured. Add [models.scoped] to config.toml.".into(),
        );
    }
    CommandResult::OpenDialog(DialogType::ScopedModels)
}

// ── Form-submit handlers ──────────────────────────────────────────────────────

pub fn run_thinking(state: &mut AppState, level: crate::model::ThinkingLevel) {
    state.config.thinking_level = level;
    state.add_system_msg(format!(
        "Thinking level set to: {}",
        state.config.thinking_level.as_str()
    ));
}
