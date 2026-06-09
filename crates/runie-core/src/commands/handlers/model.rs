use crate::commands::{CommandCategory, CommandDef, CommandHandler, CommandRegistry, CommandResult};
use crate::model::AppState;

pub fn register(registry: &mut CommandRegistry) {
    registry.register(cmd("model", "Switch model", &["m"], CommandCategory::Model, handle_model));
    registry.register(cmd("scoped-models", "Enable/disable models for cycling", &[], CommandCategory::Model, handle_scoped_models));
}

fn cmd(name: &str, desc: &str, aliases: &[&str], category: CommandCategory, handler: CommandHandler) -> CommandDef {
    CommandDef {
        name: name.into(),
        description: desc.into(),
        aliases: aliases.iter().map(|s| s.to_string()).collect(),
        category,
        handler,
        completer: None,
    }
}

fn handle_model(state: &mut AppState, args: &str) -> CommandResult {
    let rest = args.trim();
    if rest.is_empty() {
        return CommandResult::Message(format!(
            "Current model: {}/{}. Usage: /model provider/model or /model model",
            state.current_provider, state.current_model
        ));
    }
    let parts: Vec<&str> = rest.split('/').filter(|s| !s.is_empty()).collect();
    match parts.len() {
        2 => {
            state.current_provider = parts[0].to_string();
            state.current_model = parts[1].to_string();
            CommandResult::Message(format!(
                "Switched to {}/{}",
                state.current_provider, state.current_model
            ))
        }
        1 => {
            state.current_model = parts[0].to_string();
            CommandResult::Message(format!(
                "Switched to {}/{}",
                state.current_provider, state.current_model
            ))
        }
        _ => CommandResult::Message(format!(
            "Current model: {}/{}. Usage: /model provider/model or /model model",
            state.current_provider, state.current_model
        )),
    }
}

fn handle_scoped_models(_state: &mut AppState, _args: &str) -> CommandResult {
    CommandResult::Message("Scoped models: not yet implemented".into())
}
