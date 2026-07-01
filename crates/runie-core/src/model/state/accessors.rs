//! Immutable and mutable accessors for AppState.
//!
//! These methods provide the canonical read and write interface to AppState's
//! private fields. Handlers and actors use these instead of direct field access.

use crate::model::state::{
    AgentState, AppState, CompletionState, ConfigState, FffFileEntry, InputState, SessionState,
    ViewState,
};

impl AppState {
    // ── Immutable domain slice accessors ──────────────────────────────────────

    /// Immutable access to the session state slice.
    pub fn session(&self) -> &SessionState {
        &self.session
    }

    /// Immutable access to the input state slice.
    pub fn input(&self) -> &InputState {
        &self.input
    }

    /// Immutable access to the agent state slice.
    pub fn agent_state(&self) -> &AgentState {
        &self.agent
    }

    /// Immutable access to the view/cache state slice.
    pub fn view(&self) -> &ViewState {
        &self.view
    }

    /// Immutable access to the config state slice.
    pub fn config(&self) -> &ConfigState {
        &self.config
    }

    /// Immutable access to the completion/suggestion state slice.
    pub fn completion(&self) -> &CompletionState {
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

    pub fn fff_debounce(&self) -> u64 {
        self.fff_debounce
    }

    pub fn fff_file_results(&self) -> &[FffFileEntry] {
        &self.fff_file_results
    }

    pub fn permission_request_opt(&self) -> Option<&crate::model::PermissionRequestState> {
        self.perm_req.as_ref()
    }

    pub fn actor_handles(&self) -> Option<&crate::actors::LeaderHandle> {
        self.actor_handles.as_ref()
    }

    // ── Mutable accessors (crate-internal) ────────────────────────────────────

    /// Mutable access to session state.
    pub fn session_mut(&mut self) -> &mut SessionState {
        &mut self.session
    }

    pub fn input_mut(&mut self) -> &mut InputState {
        &mut self.input
    }

    pub fn agent_state_mut(&mut self) -> &mut AgentState {
        &mut self.agent
    }

    pub fn view_mut(&mut self) -> &mut ViewState {
        &mut self.view
    }

    /// Mutable access to config state (for test setup).
    pub fn config_mut(&mut self) -> &mut ConfigState {
        &mut self.config
    }

    pub(crate) fn completion_mut(&mut self) -> &mut CompletionState {
        &mut self.completion
    }

    pub fn should_quit_mut(&mut self) -> &mut bool {
        &mut self.should_quit
    }

    pub fn open_dialog_mut(&mut self) -> &mut Option<crate::commands::DialogState> {
        &mut self.open_dialog
    }

    pub fn dialog_back_stack_mut(&mut self) -> &mut Vec<crate::commands::DialogState> {
        &mut self.dialog_back_stack
    }

    pub fn login_flow_mut(&mut self) -> &mut Option<crate::login_flow::LoginFlowState> {
        &mut self.login_flow
    }

    pub(crate) fn registry_mut(&mut self) -> &mut crate::commands::CommandRegistry {
        &mut self.registry
    }

    pub fn skills(&self) -> &[crate::skills::Skill] {
        &self.skills
    }

    pub fn skills_mut(&mut self) -> &mut Vec<crate::skills::Skill> {
        &mut self.skills
    }

    pub fn prompts(&self) -> &[crate::prompts::PromptTemplate] {
        &self.prompts
    }

    pub fn prompts_mut(&mut self) -> &mut Vec<crate::prompts::PromptTemplate> {
        &mut self.prompts
    }

    pub(crate) fn trust_decisions_mut(
        &mut self,
    ) -> &mut std::collections::HashMap<std::path::PathBuf, crate::trust::TrustDecision> {
        &mut self.trust_decisions
    }

    pub fn transient_message_mut(&mut self) -> &mut Option<String> {
        &mut self.transient_message
    }

    pub fn transient_until_mut(&mut self) -> &mut Option<std::time::Instant> {
        &mut self.transient_until
    }

    pub fn transient_level_mut(&mut self) -> &mut Option<crate::event::TransientLevel> {
        &mut self.transient_level
    }

    pub fn git_info_mut(&mut self) -> &mut Option<crate::snapshot::GitInfo> {
        &mut self.git_info
    }

    pub fn cwd_name_mut(&mut self) -> &mut String {
        &mut self.cwd_name
    }

    pub(crate) fn fff_file_results_mut(&mut self) -> &mut Vec<super::FffFileEntry> {
        &mut self.fff_file_results
    }

    pub fn fff_debounce_mut(&mut self) -> &mut u64 {
        &mut self.fff_debounce
    }

    pub fn permission_request_mut(&mut self) -> &mut Option<crate::model::PermissionRequestState> {
        &mut self.perm_req
    }

    pub fn actor_handles_mut(&mut self) -> &mut Option<crate::actors::LeaderHandle> {
        &mut self.actor_handles
    }

    /// Mutable access to the authoritative turn state.
    /// Used by fact handlers to sync `AgentState` from `TurnState`.
    pub fn turn_state_mut(&mut self) -> &mut crate::actors::turn::TurnState {
        &mut self.turn_state
    }
}
