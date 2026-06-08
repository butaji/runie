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
pub mod event_bus;
pub mod orchestrator;
pub mod update;
pub mod labels;
pub mod ui;
pub mod provider;
pub mod session;
pub mod session_jsonl;
pub mod tokens;
pub mod file_refs;
pub mod fuzzy;
pub mod snapshot;
#[cfg(test)]
pub mod dsl;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod file_refs_lookup_tests;

pub use model::{AppState, ChatMessage, Role};
pub use snapshot::Snapshot;
pub use session::{Session, save, load, list, delete};
pub use session_jsonl::{
    SessionMeta, JsonlReader, JsonlWriter,
    list_session_names, delete_session, load_session, save_session,
};
pub use event::Event;
pub use labels::{
    THINKING_LOADING, thinking_with_time, thought_with_time, format_timestamp,
};
pub use ui::{Element, Feed, LazyCache};
pub use provider::{Message, Provider, ResponseChunk};
pub use tokens::{estimate_tokens, TokenTracker};
pub use file_refs::{FileRef, find_files, is_image_file, read_file_ref};
