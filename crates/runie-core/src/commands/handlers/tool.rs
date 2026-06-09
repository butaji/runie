use crate::commands::{CommandCategory, CommandDef, CommandHandler, CommandRegistry, CommandResult};
use crate::model::AppState;

pub fn register(registry: &mut CommandRegistry) {
    registry.register(cmd("readonly", "Toggle read-only mode", &["ro"], CommandCategory::Tool, handle_readonly));
    registry.register(cmd("trust", "Trust current project", &[], CommandCategory::Tool, handle_trust));
    registry.register(cmd("untrust", "Untrust current project", &[], CommandCategory::Tool, handle_untrust));
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

fn handle_readonly(_state: &mut AppState, _args: &str) -> CommandResult {
    CommandResult::Event(crate::Event::ToggleReadOnly)
}

fn handle_trust(_state: &mut AppState, _args: &str) -> CommandResult {
    CommandResult::Event(crate::Event::TrustProject)
}

fn handle_untrust(_state: &mut AppState, _args: &str) -> CommandResult {
    CommandResult::Event(crate::Event::UntrustProject)
}
