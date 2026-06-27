//! Model — Application State (mutable borrow, no cloning per event)

pub use crate::message::{now, ChatMessage, Role};
// Inner state structs are pub(crate) — accessible within runie-core but not exported externally.
// AppState itself remains pub so it can be used in public DSL signatures.
pub use crate::model::state::types::ThinkingLevel;
pub use crate::model::state::AppState;
// Re-export state types as pub for external crates.
pub use crate::model::state::agent::{AgentState, SpeedWindow};
pub use crate::model::state::input::InputState;
pub use crate::model::state::session::{CompletionState, ConfigState, SessionState};
pub use crate::model::state::types::DeliveryMode;
pub(crate) use crate::model::state::types::InputReceiver;
pub use crate::model::state::types::PermissionRequestState;
pub use crate::model::state::view::ViewState;
pub use crate::model::state::FffFileEntry;
pub use crate::model::state::ModelSource;
pub(crate) use crate::model::state::{QueuedMessage, QueuedMessageKind};
pub use crate::model_catalog::{
    build_model_selector_items, filter_models, model_catalog, ModelInfo,
};
pub use crate::scoped_model::ScopedModel;

pub use crate::snapshot::{GitInfo, Snapshot};

/// Tuple representing a single model selector entry.
pub type ModelSelectorItem = (String, String, String, bool, bool);

mod cache;
mod compaction;
pub mod state;
mod view_cache;
