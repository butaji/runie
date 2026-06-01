#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used)]

pub mod message;
pub mod tool;
pub mod event;
pub mod session;
pub mod context;
pub mod provider;
pub mod slash_command;
pub mod error;

pub use message::{Message, ToolCall, Attachment};
pub use tool::{Tool, ToolSchema, ToolOutput, ToolError};
pub use event::Event;
pub use session::{Session, MessageNode};
pub use context::{Context, WorkingMemory};
pub use provider::ProviderError;
pub use slash_command::{SlashCommand, parse_slash_command, format_help};
pub use error::RunieError;

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests;
