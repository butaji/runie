//! Runie Core - State, Events, and Update Logic
//! 
//! Following MVU (Model-View-Update) pattern:
//! - Model: AppState
//! - Events: Event enum (centralized)
//! - Update: State transitions

pub mod model;
pub mod event;
pub mod update;

pub use model::{AppState, ChatMessage};
pub use event::Event;
