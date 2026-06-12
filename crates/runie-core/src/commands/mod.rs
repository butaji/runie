//! Command Registry — unified slash commands and command palette
//!
//! # DSL Quick Reference
//!
//! ```ignore
//! // Simple message command
//! cmd!("hello", "Hello!")
//!
//! // Full command definition
//! cmd("save")
//!     .desc("Save session")
//!     .alias("s")
//!     .category(CommandCategory::Session)
//!     .form("Save", |f| f.field("Name", "session", "name"), Event::Save)
//! ```

mod dsl;
pub mod handlers;
mod registry;

pub use dsl::{CommandDef, CommandFlow, CommandResult, DialogType, CommandCategory, FormBuilder, FormField};
pub use registry::{CommandRegistry, filter_commands, DialogState};

/// Shorthand for creating commands
pub use dsl::cmd;
