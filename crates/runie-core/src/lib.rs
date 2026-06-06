//! Runie Core - State, Events, and Update Logic
//! 
//! Following MVU (Model-View-Update) pattern:
//! - Model: AppState
//! - Events: Event enum (centralized)
//! - Update: State transitions
//! - UI: DSL for declarative UI construction
//! - Labels: All static text constants

pub mod model;
pub mod event;
pub mod update;
pub mod labels;
pub mod ui;
pub mod provider;

#[cfg(test)]
mod tests;

pub use model::{AppState, ChatMessage, PANEL_CHAT, PANEL_INPUT};
pub use event::Event;
pub use labels::{
    PREFIX_USER, PREFIX_AGENT,
    THINKING_LOADING, thinking_with_time, thought_with_time,
};
pub use ui::{
    Element, Feed, LazyCache, StreamingMerge,
    format_messages, DisplayLine, DisplaySpan, Color,
};
pub use provider::{Message, Provider, ResponseChunk};
