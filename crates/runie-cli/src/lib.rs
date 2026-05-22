#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used)]

pub mod commands;
pub mod context_loader;
pub mod session_manager;
pub mod settings;

pub use commands::{Command, CommandParser};
pub use context_loader::{build_system_prompt, ContextFile, ContextLoader};
pub use session_manager::{SessionManager, SessionManagerError};
pub use settings::{Settings, CliSettings};