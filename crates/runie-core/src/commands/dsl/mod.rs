//! Command DSL Module
//!
//! Provides a fluent builder API for defining commands and their actions.
//!
//! ## Unified Command Representation
//!
//! - `Command` is the canonical runtime representation stored in the registry.
//! - `Action` is the enum describing what the command does (Handler, Form, Msg, Panel).
//!
//! This replaces the previous dual-representation pattern:
//! - Old: `CommandSpec` (static) + `CommandDef` (runtime) + `declarative::types::CommandDef` (YAML)
//! - New: Single `Command` struct with `Action` enum.

mod category;
pub(crate) mod command;
pub(crate) mod embedded_commands;
pub(crate) mod flow;
pub mod handlers;
pub(crate) mod spec;

pub use category::CommandCategory;
pub use command::{cmd, Action, Command, FormHandler};
pub use flow::{CommandFlow, CommandResult, DialogType};
// Keep re-exports for backward compatibility
pub use spec::{build_cmd, register_commands, CommandSpec, FormHandler as SpecFormHandler};
pub use spec::{CommandDef, CommandKind};
