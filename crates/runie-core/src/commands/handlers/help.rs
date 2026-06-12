//! Help commands using the new DSL

use crate::commands::{CommandCategory, CommandRegistry, CommandResult};
use crate::model::AppState;

pub fn register(registry: &mut CommandRegistry) {
    registry.register(crate::cmd!("help")
        .desc("Show available commands")
        .aliases(&["h", "?"])
        .category(CommandCategory::Help)
        .handler(handle_help));

    registry.register(crate::cmd!("quit")
        .desc("Quit application")
        .aliases(&["q", "exit"])
        .category(CommandCategory::Help)
        .handler(|_, _| CommandResult::Event(crate::Event::Quit)));
}

fn handle_help(state: &mut AppState, _: &str) -> CommandResult {
    CommandResult::Message(state.registry.help_text(&state.config.current_provider, &state.config.current_model))
}
