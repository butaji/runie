//! Subagent commands.

use crate::commands::{CommandCategory, CommandRegistry, CommandResult};
use crate::event::ControlEvent;
use crate::model::AppState;

use super::spec::{CommandKind, CommandSpec};

static SUBAGENT_COMMANDS: &[CommandSpec] = &[CommandSpec {
    name: "spawn",
    desc: "Run a subagent turn (delegated task)",
    aliases: &[],
    category: CommandCategory::System,
    sub: false,
    kind: CommandKind::Handler(handle_spawn),
}];

pub fn register(registry: &mut CommandRegistry) {
    super::spec::register_commands(registry, SUBAGENT_COMMANDS);
}

/// `/spawn <prompt>` — if a prompt is provided as an argument, emit
/// a `SpawnAgent` event directly. Otherwise, open a form to collect
/// the prompt from the user.
pub fn handle_spawn(_state: &mut AppState, args: &str) -> CommandResult {
    let prompt = args.trim();
    if prompt.is_empty() {
        return CommandResult::OpenPanelStack(Box::new(crate::commands::build_spawn_form_panel()));
    }
    CommandResult::Event(ControlEvent::SpawnAgent {
        prompt: prompt.to_string(),
    })
}
