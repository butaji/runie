//! `AppState` — the read-only UI projection of actor-owned state.
//!
//! Fields are crate-visible to allow test setup within runie-core.
//! Production code should use the accessors in `accessors.rs` for reads and
//! mutable accessors for mutations.
//!
//! The `take()` method supports `reset_session()` without requiring a full
//! struct reassignment.

use std::sync::Arc;

use super::{
    AgentState, CompletionState, ConfigState, FffFileEntry, InputState, SessionState, ViewState,
};
use crate::event::TransientLevel;
use crate::model::view_cache::ViewCache;

/// Application state — a read-only UI projection of actor-owned state.
///
/// Fields are public for test setup; production code should use accessors.
/// Inner state structs (`session`, `input`, etc.) have private fields that
/// require accessors to be added incrementally.
#[derive(Clone)]
pub struct AppState {
    // 6 inner state structs (factored domain state)
    // `session` transitions to private once all direct mutations are removed
    // (tracked in `remove-direct-appstate-mutations`). Use `session()` accessor.
    pub session: SessionState,
    pub input: InputState,
    pub agent: AgentState,
    pub view: ViewState,
    pub config: ConfigState,
    pub completion: CompletionState,

    // Singleton UI/control flags
    pub should_quit: bool,
    pub open_dialog: Option<crate::commands::DialogState>,
    pub dialog_back_stack: Vec<crate::commands::DialogState>,
    pub login_flow: Option<crate::login_flow::LoginFlowState>,
    pub registry: crate::commands::CommandRegistry,
    pub skills: Vec<crate::skills::Skill>,
    pub prompts: Vec<crate::prompts::PromptTemplate>,
    pub trust_decisions: std::collections::HashMap<std::path::PathBuf, crate::trust::TrustDecision>,
    pub transient_message: Option<String>,
    pub transient_until: Option<std::time::Instant>,
    pub transient_level: Option<crate::event::TransientLevel>,
    // NOTE: These fields are set through events (EnvDetected) in production.
    // For tests, they may be set directly via struct literals.
    pub git_info: Option<crate::snapshot::GitInfo>,
    pub cwd_name: String,
    pub fff_file_results: Vec<FffFileEntry>,
    pub fff_debounce: u64,
    pub perm_req: Option<crate::model::PermissionRequestState>,
    pub actor_handles: Option<crate::actors::ActorHandles>,
    // Cached view cache: built lazily when message_gen changes, reused across snapshots.
    pub(crate) cached_view: Option<ViewCache>,
    pub(crate) cached_view_gen: u64,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            session: SessionState::default(),
            input: InputState::default(),
            agent: AgentState::default(),
            view: ViewState::default(),
            config: ConfigState::default(),
            completion: CompletionState::default(),
            should_quit: false,
            open_dialog: None,
            dialog_back_stack: Vec::new(),
            login_flow: None,
            registry: crate::commands::CommandRegistry::new(),
            skills: Vec::new(),
            prompts: Vec::new(),
            trust_decisions: std::collections::HashMap::new(),
            transient_message: None,
            transient_until: None,
            transient_level: None,
            git_info: None,
            cwd_name: String::new(),
            fff_file_results: Vec::new(),
            fff_debounce: 0u64,
            perm_req: None,
            actor_handles: None,
            cached_view: None,
            cached_view_gen: 0,
        }
    }
}

impl AppState {
    /// Create AppState with initialized cache fields for tests.
    #[doc(hidden)]
    pub fn __with_cache_for_test() -> Self {
        let mut state = Self::default();
        state.cached_view = Some(ViewCache {
            elements: Arc::new([]),
            posts: Arc::new([]),
            line_counts: Arc::new([]),
            total_lines: 0,
            cached_gen: 0,
        });
        state.cached_view_gen = 0;
        state
    }

    /// Create a test AppState with specific transient message.
    #[doc(hidden)]
    pub fn __with_transient_test(msg: Option<String>, level: Option<crate::event::TransientLevel>) -> Self {
        let mut state = Self::__with_cache_for_test();
        *state.transient_message_mut() = msg;
        *state.transient_level_mut() = level;
        state
    }

    /// Set transient message and level for tests.
    #[doc(hidden)]
    pub fn __set_transient_for_test(&mut self, msg: Option<String>, level: Option<crate::event::TransientLevel>) {
        *self.transient_message_mut() = msg;
        *self.transient_level_mut() = level;
    }

    /// Swap out all fields to `Default`, returning the old values.
    /// Used by `reset_session()` to preserve select fields.
    pub(crate) fn take(&mut self) -> AppState {
        let mut prev = AppState::default();
        std::mem::swap(self, &mut prev);
        prev
    }
}
