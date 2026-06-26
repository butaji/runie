//! Application state types.

// Inner state structs (moved from src/state/)
pub mod agent;
pub mod input;
pub mod session;
mod session_restore;
pub mod view;

// Original state module files
mod accessors;
mod app_state;
mod domain_ops;
mod helpers;
mod ranking;
pub mod types;

// State types are crate-private — only re-exported within runie-core.
pub(crate) use agent::AgentState;
pub use app_state::AppState;
pub(crate) use input::{CommandUsage, InputState};
pub use session::{CompletionState, ConfigState, ModelSource, SessionState};
pub use types::FffFileEntry;
pub(crate) use types::{QueuedMessage, QueuedMessageKind};
pub(crate) use view::ViewState;
