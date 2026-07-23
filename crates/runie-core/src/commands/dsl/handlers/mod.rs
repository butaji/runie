//! Command handlers — execution logic for slash commands.
//!

pub mod ask;
pub mod goal;
pub mod help;
pub mod macros_;
pub mod mode;
pub mod model;
pub mod registry;
pub mod session;
pub mod status;
pub mod swarm;
pub mod system;
pub mod tool;

use crate::commands::CommandRegistry;
pub use registry::{HandlerRegistry, NamedHandler};

/// Global handler registry — maps command names to handler functions.
/// This is used by YAML-loaded commands to look up their handlers.
pub static HANDLER_REGISTRY: std::sync::LazyLock<HandlerRegistry> = std::sync::LazyLock::new(init_handler_registry);

fn init_handler_registry() -> HandlerRegistry {
    let mut registry = HandlerRegistry::new();

    // Register all handlers
    session::register_handlers(&mut registry);
    model::register_handlers(&mut registry);
    mode::register_handlers(&mut registry);
    tool::register_handlers(&mut registry);
    system::register_handlers(&mut registry);
    help::register_handlers(&mut registry);
    status::register_handlers(&mut registry);
    registry.register("goal", NamedHandler::Handler(goal::handle_goal));
    registry.register("ask", NamedHandler::Handler(ask::handle_ask));
    registry.register("swarm", NamedHandler::Handler(swarm::handle_swarm));

    // Register built-in handlers that are defined inline in model.rs
    registry.register("model", NamedHandler::Handler(model::handle_model));
    registry.register("thinking", NamedHandler::Handler(model::handle_thinking));
    registry.register(
        "scoped_models",
        NamedHandler::Handler(model::handle_scoped_models),
    );

    registry
}

/// Register all handlers from the static tables.
/// Commands are now loaded from YAML; this is a stub for backward compat.
pub fn register_all(_registry: &mut CommandRegistry) {
    // All commands are now loaded from YAML resources/commands/*.yaml
    // via CommandRegistry::with_commands() — static tables have been removed.
}
