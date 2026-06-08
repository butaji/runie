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
pub mod tokens;
pub mod file_refs;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod file_refs_lookup_tests;

pub use model::{AppState, ChatMessage, Color, Role, Snapshot, PANEL_CHAT, PANEL_INPUT};
pub use session::{Session, save, load, list, delete};
pub use event::Event;
pub use labels::{
    PREFIX_USER, PREFIX_AGENT,
    THINKING_LOADING, thinking_with_time, thought_with_time,
};
pub use ui::{Element, Feed, LazyCache};
pub use provider::{Message, Provider, ResponseChunk};
pub use tokens::{estimate_tokens, TokenTracker};
pub use file_refs::{FileRef, find_files, is_image_file, read_file_ref};
