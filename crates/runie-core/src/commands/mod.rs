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

pub use dsl::{
    CommandCategory, CommandDef, CommandFlow, CommandResult, DialogType, FormBuilder, FormField,
};
pub use registry::{filter_commands, CommandRegistry, DialogState};

/// Shorthand for creating commands
pub use dsl::cmd;
