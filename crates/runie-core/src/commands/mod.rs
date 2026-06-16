//! Command Registry — unified slash commands and command palette
//!
//! # DSL Quick Reference
//!
//! ```
//! use runie_core::commands::CommandCategory;
//! use runie_core::Event;
//! use runie_core::event::{CommandEvent, LoginFlowEvent};
//!
//! // Simple message command
//! let _ = runie_core::cmd!("hello", "Hello!");
//!
//! // Full command definition
//! let _ = runie_core::commands::cmd("save")
//!     .desc("Save session")
//!     .alias("s")
//!     .category(CommandCategory::Session)
//!     .form("Save", |f| {
//!         f.field("Name", "session", "name").on_submit(|values| {
//!             CommandEvent::RunSaveCommand {
//!                 name: values.get("name").cloned().unwrap_or_default(),
//!             }
//!         })
//!     });
//! ```

pub mod agents_manager;
pub mod dsl;
mod registry;
#[cfg(test)]
mod tests;

pub use dsl::{
    build_spawn_form_panel, CommandCategory, CommandDef, CommandFlow, CommandResult, DialogType,
};
pub use registry::{filter_commands, CommandRegistry, DialogState};

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
