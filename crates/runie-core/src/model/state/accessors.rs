//! Immutable and mutable accessors for AppState.
//!
//! These methods provide the canonical read and write interface to AppState's
//! private fields. Handlers and actors use these instead of direct field access.

use crate::model::state::{
    AgentState, AppState, CompletionState, ConfigState, InputState, SessionState,
    ViewState,
};

impl AppState {
    // ── Immutable domain slice accessors ──────────────────────────────────────

    /// Immutable access to the session state slice.
    pub(crate) fn session(&self) -> &SessionState {
        &self.session
    }

    /// Immutable access to the input state slice.
    pub(crate) fn input(&self) -> &InputState {
        &self.input
    }

    /// Immutable access to the agent state slice.
    pub(crate) fn agent_state(&self) -> &AgentState {
        &self.agent
    }

    /// Immutable access to the view/cache state slice.
    pub(crate) fn view(&self) -> &ViewState {
        &self.view
    }

    /// Immutable access to the config state slice.
    pub(crate) fn config(&self) -> &ConfigState {
        &self.config
    }

    /// Immutable access to the completion/suggestion state slice.
    pub(crate) fn completion(&self) -> &CompletionState {
        &self.completion
    }

    // ── Immutable UI/control flag accessors ────────────────────────────────────

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn open_dialog(&self) -> Option<&crate::commands::DialogState> {
        self.open_dialog.as_ref()
    }

    pub fn dialog_back_stack(&self) -> &[crate::commands::DialogState] {
        &self.dialog_back_stack
    }

    pub fn login_flow(&self) -> Option<&crate::login_flow::LoginFlowState> {
        self.login_flow.as_ref()
    }

    pub fn registry(&self) -> &crate::commands::CommandRegistry {
        &self.registry
    }

    pub fn trust_decisions(
        &self,
    ) -> &std::collections::HashMap<std::path::PathBuf, crate::trust::TrustDecision> {
        &self.trust_decisions
    }

    pub fn transient_message(&self) -> Option<&String> {
        self.transient_message.as_ref()
    }

    pub fn transient_until(&self) -> Option<std::time::Instant> {
        self.transient_until
    }

    pub fn transient_level(&self) -> Option<crate::event::TransientLevel> {
        self.transient_level
    }

    pub fn git_info(&self) -> Option<&crate::snapshot::GitInfo> {
        self.git_info.as_ref()
    }

    pub fn cwd_name(&self) -> &str {
        &self.cwd_name
    }

    pub(crate) fn fff_file_results(&self) -> &[super::FffFileEntry] {
        &self.fff_file_results
    }

    pub fn fff_debounce(&self) -> u32 {
        self.fff_debounce
    }

    pub fn permission_request(&self) -> Option<&crate::model::PermissionRequestState> {
        self.permission_request.as_ref()
    }

    pub fn approval_registry(
        &self,
    ) -> &std::sync::Arc<std::sync::Mutex<crate::permissions::ApprovalRegistry>> {
        &self.approval_registry
    }

    pub fn actor_handles(&self) -> Option<&crate::actors::ActorHandles> {
        self.actor_handles.as_ref()
    }

    pub fn config_cache(&self) -> Option<&crate::config::Config> {
        self.config_cache.as_ref()
    }

    // ── Mutable accessors (crate-internal) ────────────────────────────────────

    /// Mutable access to session state for handlers within `runie-core`.
    pub(crate) fn session_mut(&mut self) -> &mut SessionState {
        &mut self.session
    }

    pub(crate) fn input_mut(&mut self) -> &mut InputState {
        &mut self.input
    }

    pub(crate) fn agent_state_mut(&mut self) -> &mut AgentState {
        &mut self.agent
    }

    pub(crate) fn view_mut(&mut self) -> &mut ViewState {
        &mut self.view
    }

    pub(crate) fn config_mut(&mut self) -> &mut ConfigState {
        &mut self.config
    }

    pub(crate) fn completion_mut(&mut self) -> &mut CompletionState {
        &mut self.completion
    }
}
