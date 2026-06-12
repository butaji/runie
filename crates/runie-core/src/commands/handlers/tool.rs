//! Tool commands using the new DSL

use crate::commands::{cmd, CommandCategory, CommandRegistry, CommandResult};
use crate::model::AppState;

pub fn register(registry: &mut CommandRegistry) {
    registry.register(crate::cmd!("readonly")
        .desc("Toggle read-only mode")
        .aliases(&["ro"])
        .category(CommandCategory::Tool)
        .handler(|_, _| CommandResult::Event(crate::Event::ToggleReadOnly)));

    registry.register(crate::cmd!("trust")
        .desc("Trust current project")
        .category(CommandCategory::Tool)
        .handler(|_, _| CommandResult::Event(crate::Event::TrustProject)));

    registry.register(crate::cmd!("untrust")
        .desc("Untrust current project")
        .category(CommandCategory::Tool)
        .handler(|_, _| CommandResult::Event(crate::Event::UntrustProject)));
}
