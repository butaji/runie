//! Safety and permission commands.

use crate::commands::dsl::handlers::NamedHandler;
use crate::commands::CommandResult;
use crate::model::AppState;

/// Register all tool handlers with the handler registry (for YAML-based commands).
pub fn register_handlers(registry: &mut crate::commands::dsl::handlers::registry::HandlerRegistry) {
    registry.register(
        "readonly",
        NamedHandler::Handler(|_: &mut AppState, _: &str| {
            CommandResult::Event(crate::Event::ToggleReadOnly)
        }),
    );
    registry.register(
        "trust",
        NamedHandler::Handler(|_: &mut AppState, _: &str| {
            CommandResult::Event(crate::Event::TrustProject)
        }),
    );
    registry.register(
        "untrust",
        NamedHandler::Handler(|_: &mut AppState, _: &str| {
            CommandResult::Event(crate::Event::UntrustProject)
        }),
    );
}
