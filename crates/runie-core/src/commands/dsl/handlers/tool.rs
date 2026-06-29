//! Safety and permission commands.

use crate::commands::CommandResult;
use crate::model::AppState;

/// Register all tool handlers with the handler registry (for YAML-based commands).
pub fn register_handlers(registry: &mut crate::commands::dsl::handlers::registry::HandlerRegistry) {
    use crate::register_handler;
    register_handler!(registry, "readonly", Handler(|_: &mut AppState, _: &str| {
        CommandResult::Event(crate::Event::ToggleReadOnly)
    }));
    register_handler!(registry, "trust", Handler(|_: &mut AppState, _: &str| {
        CommandResult::Event(crate::Event::TrustProject)
    }));
    register_handler!(registry, "untrust", Handler(|_: &mut AppState, _: &str| {
        CommandResult::Event(crate::Event::UntrustProject)
    }));
}
