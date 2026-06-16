//! Command handlers — delegates to dsl/handlers/.
//!
//! Re-exports everything from the unified `dsl::handlers/` location.

use crate::commands::CommandRegistry;

pub mod agents;
pub mod help;
pub mod model;
pub mod session; // re-exported from dsl::handlers::session
pub mod subagent;
pub mod system;
pub mod tool;

// Delegates to dsl::spec so handler modules can use `super::spec::CommandSpec`.
pub mod spec {
    pub use crate::commands::dsl::spec::{
        build_cmd, register_commands, CommandKind, CommandSpec, FormSubmitFn,
    };
}

pub fn register_all(registry: &mut CommandRegistry) {
    session::register(registry);
    model::register(registry);
    tool::register(registry);
    system::register(registry);
    subagent::register(registry);
    agents::register(registry);
    help::register(registry);
}
