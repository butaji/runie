//! Safety and permission commands.

use super::spec::{CommandKind, CommandSpec};
use crate::commands::{CommandCategory, CommandRegistry, CommandResult};
use crate::model::AppState;

fn toggle_readonly(_: &mut AppState, _: &str) -> CommandResult {
    CommandResult::Event(crate::event::ModelConfigEvent::ToggleReadOnly)
}
fn trust_project(_: &mut AppState, _: &str) -> CommandResult {
    CommandResult::Event(crate::event::ModelConfigEvent::TrustProject)
}
fn untrust_project(_: &mut AppState, _: &str) -> CommandResult {
    CommandResult::Event(crate::event::ModelConfigEvent::UntrustProject)
}

static SAFETY_COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        name: "readonly",
        desc: "Toggle read-only mode",
        aliases: &["ro"],
        category: CommandCategory::Safety,
        sub: false,
        kind: CommandKind::Handler(toggle_readonly),
    },
    CommandSpec {
        name: "trust",
        desc: "Trust current project",
        aliases: &[],
        category: CommandCategory::Safety,
        sub: false,
        kind: CommandKind::Handler(trust_project),
    },
    CommandSpec {
        name: "untrust",
        desc: "Untrust current project",
        aliases: &[],
        category: CommandCategory::Safety,
        sub: false,
        kind: CommandKind::Handler(untrust_project),
    },
];

pub fn register(registry: &mut CommandRegistry) {
    super::spec::register_commands(registry, SAFETY_COMMANDS);
}
