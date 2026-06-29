//! Command DSL Module
//!
//! Provides a fluent builder API for defining commands and their flows.
//! Command implementations (handlers) live in the `handlers/` sub-module.
//!
//! `CommandSpec` is the static table format (borrowed strings, no heap in static).
//! `CommandDef` is the runtime-owned version stored in the registry.
//! `build_cmd()` converts a `CommandSpec` to a `CommandDef`.

mod category;
pub(crate) mod embedded_commands;
mod flow;
pub mod handlers;
pub(crate) mod spec;

pub use category::CommandCategory;
pub use flow::{CommandFlow, CommandResult, DialogType};
pub use spec::{build_cmd, register_commands, CommandDef, CommandKind, CommandSpec, FormHandler};

/// Shorthand constructor — equivalent to `CommandDef::new(name)`.
pub fn cmd(name: &'static str) -> CommandDef {
    CommandDef::new(name)
}
