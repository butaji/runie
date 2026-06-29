//! Safety and permission commands.

use crate::commands::{CommandCategory, CommandRegistry, CommandResult};
use crate::model::AppState;

use crate::commands::dsl::spec::{build_cmd, CommandKind, CommandSpec};

static COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        name: "readonly",
        desc: "Toggle read-only mode",
        aliases: &["ro"],
        category: CommandCategory::Safety,
        sub: false,
        kind: CommandKind::Handler(|_: &mut AppState, _: &str| {
            CommandResult::Event(crate::Event::ToggleReadOnly)
        }),
    },
    CommandSpec {
        name: "trust",
        desc: "Trust current project",
        aliases: &[],
        category: CommandCategory::Safety,
        sub: false,
        kind: CommandKind::Handler(|_: &mut AppState, _: &str| {
            CommandResult::Event(crate::Event::TrustProject)
        }),
    },
    CommandSpec {
        name: "untrust",
        desc: "Untrust current project",
        aliases: &[],
        category: CommandCategory::Safety,
        sub: false,
        kind: CommandKind::Handler(|_: &mut AppState, _: &str| {
            CommandResult::Event(crate::Event::UntrustProject)
        }),
    },
];

pub fn register(registry: &mut CommandRegistry) {
    for spec in COMMANDS {
        registry.register(build_cmd(spec));
    }
}
