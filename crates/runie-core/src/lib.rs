//! Runie Core — State, Events, Update, UI Architecture
//!
//! Architecture (three layers):
//!   model     :: AppState, ChatMessage (source of truth)
//!   event     :: Event enum (all possible state transitions)
//!   update    :: State transitions (pure functions)
//!   ui        :: Elements, Transform (view layer)
//!   labels    :: Static text constants

pub mod model;
pub mod event;
pub mod update;
pub mod labels;
pub mod ui;
pub mod provider;
pub mod session;

#[cfg(test)]
mod tests;

pub use model::{AppState, ChatMessage, Color, Role, PANEL_CHAT, PANEL_INPUT};
pub use session::{Session, save, load, list, delete};
pub use event::Event;
pub use labels::{
    PREFIX_USER, PREFIX_AGENT,
    THINKING_LOADING, thinking_with_time, thought_with_time,
};
pub use ui::{
    Element, Feed, LazyCache, StreamingMerge,
    format_messages, DisplayLine, DisplaySpan,
};
pub use provider::{Message, Provider, ResponseChunk};
