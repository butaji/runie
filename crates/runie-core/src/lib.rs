//! Runie Core - State, Events, and Update Logic
//! 
//! Following MVU (Model-View-Update) pattern:
//! - Model: AppState, ChatMessage
//! - Events: Event enum
//! - Update: State transitions

pub mod model;
pub mod update;

pub use model::{AppState, ChatMessage};
pub use update::Event;
