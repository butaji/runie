//! Command Registry — unified slash commands and command palette
//!
//! # DSL Quick Reference
//!
//! ```
//! use runie_core::commands::{CommandCategory, cmd};
//! use runie_core::Event;
//!
//! // Simple message command
//! let _ = cmd("hello").msg("Hello!");
//!
//! // Full command definition
//! let _ = cmd("save")
//!     .desc("Save session")
//!     .alias("s")
//!     .category(CommandCategory::Session)
//!     .form("Save", |f| {
//!         f.field("Name", "session", "name").on_submit(|values| {
//!             Event::RunSaveCommand {
//!                 name: values.get("name").cloned().unwrap_or_default(),
//!             }
//!         })
//!     });
//! ```

pub mod dsl;
mod registry;
#[cfg(test)]
mod tests;

pub use dsl::{CommandCategory, CommandDef, CommandFlow, CommandResult, DialogType};
pub use registry::{filter_commands, CommandRegistry, DialogKind, DialogState};

/// Shorthand for creating commands
pub use dsl::cmd;

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
