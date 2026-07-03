//! Command Registry — unified slash commands and command palette
//!
//! # Unified Command Model
//!
//! - `Command` is the canonical runtime representation stored in the registry.
//! - `Action` is the enum describing what the command does.
//!
//! ```
//! use runie_core::commands::{CommandCategory, cmd};
//! use runie_core::Event;
//!
//! // Simple message command
//! let _ = cmd("hello").msg("Hello!");
//!
//! // Form-based command: submit routes through command registry via form_with_handler
//! use runie_core::commands::CommandResult;
//! let _ = cmd("save")
//!     .desc("Save session")
//!     .alias("s")
//!     .category(CommandCategory::Session)
//!     .form_with_handler(
//!         "Save Session",
//!         |f| f.field("Name", "session", "name"),
//!         |_state, _args| CommandResult::None,
//!     );
//! ```

pub mod dsl;
mod registry;
#[cfg(test)]
mod tests;

pub use dsl::{
    cmd, Action, Command, CommandCategory, CommandDef, CommandFlow, CommandResult, DialogType,
    FormHandler,
};
pub use registry::{filter_commands, CommandRegistry, DialogKind, DialogState};

/// One row in the command palette.
#[derive(Debug, Clone)]
pub struct CommandRow {
    pub category: String,
    pub name: String,
    pub desc: String,
    pub event: crate::Event,
}

impl CommandRow {
    pub fn new(
        category: impl Into<String>,
        name: impl Into<String>,
        desc: impl Into<String>,
        event: crate::Event,
    ) -> Self {
        Self {
            category: category.into(),
            name: name.into(),
            desc: desc.into(),
            event,
        }
    }
}
