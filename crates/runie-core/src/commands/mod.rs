//! Command Registry — unified slash commands and command palette
//!
//! # DSL Quick Reference
//!
//! ```
//! use runie_core::commands::CommandCategory;
//! use runie_core::Event;
//!
//! // Simple message command
//! let _ = runie_core::cmd!("hello", "Hello!");
//!
//! // Full command definition
//! let _ = runie_core::commands::cmd("save")
//!     .desc("Save session")
//!     .alias("s")
//!     .category(CommandCategory::Session)
//!     .form(
//!         "Save",
//!         |f| f.field("Name", "session", "name"),
//!         Event::RunSaveCommand { name: String::new() },
//!     );
//! ```

mod dsl;
pub mod handlers;
mod registry;
#[cfg(test)]
mod tests;
pub mod agents_manager;

pub use dsl::{
    CommandCategory, CommandDef, CommandFlow, CommandResult, DialogType, FormBuilder, FormField,
};
pub use registry::{filter_commands, CommandRegistry, DialogState};

/// Shorthand for creating commands
pub use dsl::cmd;

/// One row in the command palette.
///
/// Structured like [`dialog::builders::SettingsRow`](crate::dialog::builders::SettingsRow):
/// the palette builder receives typed rows and decides how to render them,
/// instead of parsing a combined "name description" string.
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
