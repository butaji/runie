//! Subagent commands.

use crate::commands::{CommandCategory, CommandRegistry, CommandResult};
use crate::event::Event;
use crate::model::AppState;

pub fn register(registry: &mut CommandRegistry) {
    registry.register(crate::cmd!("spawn")
        .desc("Run a subagent turn (delegated task)")
        .category(CommandCategory::System)
        .handler(handle_spawn));
}

/// `/spawn <prompt>` — emit a `SpawnAgent` event. The binary layer
/// (runie-term) catches the event, runs the subagent via
/// `runie_agent::subagent::run_subagent`, and emits the result as a
/// system message.
///
/// We keep the actual subagent execution in the binary layer so that
/// `runie-core` doesn't need to depend on `runie-agent` (which depends
/// back on it).
fn handle_spawn(_state: &mut AppState, args: &str) -> CommandResult {
    let prompt = args.trim();
    if prompt.is_empty() {
        return CommandResult::Message("Usage: /spawn <prompt>".into());
    }
    CommandResult::Event(Event::SpawnAgent { prompt: prompt.to_string() })
}
