//! Command handlers — execution logic and panel builders for each command group.
//!
//! One module per command category. Each module:
//!   - defines a `static COMMANDS: &[CommandSpec]` table
//!   - exposes `pub fn register(registry: &mut CommandRegistry)`

pub mod help;
pub mod model;
pub mod session;
pub mod system;
pub mod tool;

pub fn register_all(registry: &mut crate::commands::CommandRegistry) {
    session::register(registry);
    model::register(registry);
    tool::register(registry);
    system::register(registry);
    help::register(registry);
}
