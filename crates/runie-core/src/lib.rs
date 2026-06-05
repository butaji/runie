//! Runie Core - State, Events, and Update Logic
//! 
//! Following MVU (Model-View-Update) pattern:
//! - Model: AppState
//! - Events: Event enum (centralized)
//! - Update: State transitions
//! - Format: Message formatting
//! - Labels: All static text constants

pub mod model;
pub mod event;
pub mod update;
pub mod format;
pub mod labels;

pub use model::{AppState, ChatMessage};
pub use event::Event;
pub use format::{
    format_messages, user_message, agent_answer, thinking, thought_message,
    DisplayLine, DisplaySpan, Color,
};
pub use labels::{
    PANEL_CHAT, PANEL_INPUT, PREFIX_USER, PREFIX_AGENT,
    THINKING_LOADING, thinking_with_time, thought_with_time,
};
