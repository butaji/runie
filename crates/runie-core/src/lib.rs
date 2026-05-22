pub mod message;
pub mod tool;
pub mod event;
pub mod session;
pub mod context;
pub mod compactor;
pub mod provider;
pub mod slash_command;

pub use message::{Message, ToolCall, Attachment};
pub use tool::{Tool, ToolSchema, ToolOutput, ToolError};
pub use event::Event;
pub use session::{Session, MessageNode};
pub use context::{Context, WorkingMemory};
pub use compactor::{Compactor, CompactorError, SimpleCompactor};
pub use provider::ProviderError;
pub use slash_command::{SlashCommand, parse_slash_command, format_help};

#[cfg(test)]
mod tests;
