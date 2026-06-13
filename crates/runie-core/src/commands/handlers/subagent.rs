//! Subagent commands.

use crate::commands::{CommandCategory, CommandRegistry, CommandResult};
use crate::event::Event;
use crate::model::AppState;

pub fn register(registry: &mut CommandRegistry) {
    registry.register(
        crate::cmd!("spawn")
            .desc("Run a subagent turn (delegated task)")
            .category(CommandCategory::System)
            .handler(handle_spawn),
    );
}

/// `/spawn <prompt>` — if a prompt is provided as an argument, emit
/// a `SpawnAgent` event directly. Otherwise, open a form to collect
/// the prompt from the user.
///
/// The binary layer (runie-term) catches the event, runs the subagent
/// via `runie_agent::subagent::run_subagent`, and emits the result as
/// a system message.
///
/// We keep the actual subagent execution in the binary layer so that
/// `runie-core` doesn't need to depend on `runie-agent` (which depends
/// back on it).
pub fn handle_spawn(_state: &mut AppState, args: &str) -> CommandResult {
    let prompt = args.trim();
    if prompt.is_empty() {
        // No args: open a form to collect the prompt.
        // Don't return a "Usage:" message — that pollutes the chat feed.
        return CommandResult::OpenPanelStack(crate::commands::build_spawn_form_panel());
    }
    CommandResult::Event(Event::SpawnAgent {
        prompt: prompt.to_string(),
    })
}
