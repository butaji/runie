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
pub(crate) mod yaml;

pub use category::CommandCategory;
pub use command::{cmd, Action, Command, FormHandler};
pub use flow::{CommandFlow, CommandResult, DialogType};
// Legacy re-exports still needed by yaml.rs
pub use spec::{CommandKind, CommandDef, CommandSpec, FormHandler as SpecFormHandler};
