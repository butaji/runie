//! Command DSL Module
//!
//! Provides a fluent builder API for defining commands and their flows.
//! Command implementations (handlers) live in the `handlers/` sub-module.
//!
//! `CommandSpec` is the static table format (borrowed strings, no heap in static).
//! `CommandDef` is the runtime-owned version stored in the registry.
//! `build_cmd()` converts a `CommandSpec` to a `CommandDef`.

pub(crate) mod spec;
mod flow;
mod category;
pub mod handlers;

pub use spec::{build_cmd, register_commands, CommandDef, CommandKind, CommandSpec, FormHandler};
pub use flow::{CommandFlow, CommandResult, DialogType};
pub use category::CommandCategory;

/// Shorthand constructor — equivalent to `CommandDef::new(name)`.
pub fn cmd(name: &'static str) -> CommandDef {
    CommandDef::new(name)
}
