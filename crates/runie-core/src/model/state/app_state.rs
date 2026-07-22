//! `AppState` — the read-only UI projection of actor-owned state.
//!
//! Fields are crate-visible to allow test setup within runie-core.
//! Production code should use the accessors in `accessors.rs` for reads and
//! mutable accessors for mutations.
//!
//! The `take()` method supports `reset_session()` without requiring a full
//! struct reassignment.

use super::{AgentState, CompletionState, ConfigState, FffFileEntry, InputState, SessionState, ViewState};
use runie_patterns::swarm::{OrphanedWorkerTracker, StatusCounts};

/// Application state — a read-only UI projection of actor-owned state.
///
/// Fields are public for test setup; production code should use accessors.
/// Inner state structs (`session`, `input`, etc.) have private fields that
/// require accessors to be added incrementally.
///
/// NOTE: View projection (Element/Post/Feed) is no longer cached in AppState.
/// The projection is built on-demand by `ensure_fresh()` and stored in `Snapshot`.
/// UiActor owns the Element cache for rendering purposes.
#[derive(Clone, Default)]
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
    /// True when the command palette was opened from the chat-input "/"
    /// autocomplete (rather than the persistent Ctrl+P command bar). An
    /// autocomplete palette is ephemeral: activating a command returns to the
    /// chat input instead of back to the palette, so the next "/" opens a fresh
    /// palette. Reset to `false` on every palette open; the autocomplete open
    /// paths flip it to `true` right after opening.
    pub command_palette_from_input: bool,
    pub login_flow: Option<crate::login_flow::LoginFlowState>,
    pub registry: crate::commands::CommandRegistry,
    pub skills: Vec<crate::skills::Skill>,
    pub mcp_servers: Vec<crate::dialog::builders::McpServerRow>,
    pub prompts: Vec<crate::prompts::PromptTemplate>,
    pub trust_decisions: indexmap::IndexMap<camino::Utf8PathBuf, crate::trust::TrustDecision>,
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
    pub goal_state: Option<crate::model::GoalState>,
    pub question_state: Option<crate::model::QuestionState>,
    pub actor_handles: Option<crate::actors::LeaderHandle>,
    /// Optional event bus bridge so core state handlers can publish events that
    /// must be observed by the actor layer (e.g. a form-generated `SubmitKey`).
    pub(crate) event_bus: Option<crate::bus::EventBus<crate::Event>>,
    /// Set once the onboarding/login flow has been started for this session.
    /// Prevents `--mock-onboarding` from re-opening the provider picker after
    /// the user completes the flow and the saved config is loaded.
    pub(crate) onboarding_started: bool,
    /// Separate counter for session message IDs, independent of TurnActor's `next_id`.
    /// AppState generates IDs for session messages; TurnActor generates IDs for
    /// request queue messages. These are kept separate to avoid double-increment.
    pub(crate) session_msg_id: u64,
    /// Role context for pending model selection in mode selector.
    /// When a lead/worker model is being picked from the `/mode` dialog, this
    /// field holds the role (`"lead"` or `"worker"`) so `SwitchModelWithLevel`
    /// knows which config to update.
    pub(crate) pending_model_role: Option<String>,
    /// Swarm worker tracker for orphan reconciliation (Task 26).
    pub swarm_state: OrphanedWorkerTracker,
    /// True when the swarm circuit breaker has tripped (dispatch paused).
    pub circuit_breaker_tripped: bool,
    /// Threshold that triggered the circuit breaker (for display).
    pub circuit_breaker_threshold: u32,
}

impl AppState {
    /// Create a test AppState with specific transient message.
    #[doc(hidden)]
    pub fn __with_transient_test(msg: Option<String>, level: Option<crate::event::TransientLevel>) -> Self {
        let mut state = Self::default();
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

    /// Queue a steering (follow-up) message and update input history.
    /// Used by UiActor::dispatch_submit_content to keep AppState's input_history
    /// in sync with the session history sent to SessionActor.
    pub fn queue_steering_and_update_history(&mut self, content: String) {
        use crate::actors::TurnMsg;
        let handles = self.actor_handles().cloned();
        if let Some(ref h) = handles {
            let _ = h
                .turn
                .try_send(TurnMsg::QueueFollowUp { content: content.clone() });
        } else {
            // Test mode: update AgentState directly (no TurnActor in tests).
            self.agent_state_mut()
                .message_queue
                .push(crate::model::QueuedMessage {
                    content: content.clone(),
                    kind: crate::model::QueuedMessageKind::Steering,
                });
        }
        self.view_mut().scroll = 0;
        self.view_mut().dirty = true;
        self.push_to_input_history(&content);
    }

    /// Submit a user message and update input history.
    /// Used by UiActor::dispatch_submit_content to keep AppState's input_history
    /// in sync with the session history sent to SessionActor.
    pub fn submit_user_message_and_update_history(&mut self, content: String) {
        let history_content = content.clone();
        self.submit_user_message(content);
        self.push_to_input_history(&history_content);
    }

    /// Clean up orphaned and cancelled swarm workers.
    /// Returns the counts of cleaned workers.
    pub fn swarm_cleanup(&self) -> StatusCounts {
        let before = self.swarm_state.status_counts();
        let _ = self.swarm_state.cleanup_orphaned_workers();
        let after = self.swarm_state.status_counts();
        StatusCounts {
            running: before.running - after.running,
            completed: before.completed - after.completed,
            failed: before.failed - after.failed,
            cancelled: before.cancelled - after.cancelled,
            orphaned: before.orphaned - after.orphaned,
        }
    }

    /// Get swarm status counts.
    pub fn swarm_status_counts(&self) -> StatusCounts {
        self.swarm_state.status_counts()
    }
}
