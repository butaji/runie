use crate::commands::{CommandCategory, CommandDef, CommandHandler, CommandRegistry, CommandResult};
use crate::model::AppState;

pub fn register(registry: &mut CommandRegistry) {
    registry.register(cmd("help", "Show available commands", &["h", "?"], CommandCategory::Help, handle_help));
    registry.register(cmd("quit", "Quit application", &["q", "exit"], CommandCategory::Help, handle_quit));
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

fn handle_help(state: &mut AppState, _args: &str) -> CommandResult {
    let text = state.registry.help_text(&state.config.current_provider, &state.config.current_model);
    CommandResult::Message(text)
}

fn handle_quit(_state: &mut AppState, _args: &str) -> CommandResult {
    CommandResult::Event(crate::Event::Quit)
}
