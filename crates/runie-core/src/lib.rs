//! Runie Core - State, Events, and Update Logic
//! 
//! Following MVU (Model-View-Update) pattern:
//! - Model: AppState
//! - Events: Event enum (centralized)
//! - Update: State transitions
//! - Format: Message formatting

pub mod model;
pub mod event;
pub mod update;
pub mod format;

pub use model::{AppState, ChatMessage};
pub use event::Event;
pub use format::{format_messages, DisplayLine, DisplaySpan, Color};
