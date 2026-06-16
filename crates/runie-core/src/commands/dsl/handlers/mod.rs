//! Command handlers — execution logic and panel builders for each command group.
//!
//! One module per command category. Each module:
//!   - defines a `static COMMANDS: &[CommandSpec]` table
//!   - exposes `pub fn register(registry: &mut CommandRegistry)`

pub mod agents;
pub mod help;
pub mod model;
pub mod session;
pub mod subagent;
pub mod system;
pub mod tool;

/// Re-export spec so handler modules can use `super::spec::CommandSpec`.
pub mod spec {
    pub use crate::commands::dsl::spec::{
        build_cmd, register_commands, CommandKind, CommandSpec, FormSubmitFn,
    };
}

pub fn register_all(registry: &mut crate::commands::CommandRegistry) {
    session::register(registry);
    model::register(registry);
    tool::register(registry);
    system::register(registry);
    subagent::register(registry);
    agents::register(registry);
    help::register(registry);
}
