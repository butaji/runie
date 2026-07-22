//! Goal Mode Orchestration.
//!
//! Manages multi-phase goal execution with planning, execution, and verification:
//! - GoalPhase: Idle, Planning, Executing, Paused, Completed, Failed
//! - GoalTracker: State machine with transitions
//! - Subagent roles: planner, worker, verifier, strategist

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Notify, RwLock};
use tracing::{debug, info, warn};

/// Goal lifecycle phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum GoalPhase {
    /// No active goal.
    #[default]
    Idle,
    /// Planner subagent is writing plan.md.
    Planning,
    /// Worker subagents are implementing the goal.
    Executing,
    /// Goal is paused (user or automatic).
    Paused,
    /// Goal achieved and verified.
    Completed,
    /// Goal failed or budget exhausted.
    Failed,
}


/// Goal status (why the goal is where it is).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum GoalStatus {
    /// Goal is actively running.
    #[default]
    Active,
    /// User paused (Ctrl+C or /goal pause).
    UserPaused,
    /// Verification run cap exhausted.
    BackOffPaused,
    /// Verifier flagged same gaps repeatedly.
    NoProgressPaused,
    /// Infrastructure error occurred.
    InfraPaused,
    /// Environment contradiction after N turns.
    Blocked,
    /// Token budget exceeded.
    BudgetLimited,
    /// Goal achieved and verified.
    Complete,
}

impl GoalStatus {
    /// Check if status indicates a paused goal.
    pub fn is_paused(&self) -> bool {
        matches!(
            self,
            GoalStatus::UserPaused
                | GoalStatus::BackOffPaused
                | GoalStatus::NoProgressPaused
                | GoalStatus::InfraPaused
                | GoalStatus::Blocked
        )
    }
}


/// Subagent role in goal execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GoalRole {
    /// Writes plan.md.
    Planner,
    /// Implements the goal.
    Worker,
    /// Verifies goal completion.
    Verifier,
    /// Diagnoses stuck runs.
    Strategist,
    /// Summarizes completed goals.
    Summarizer,
}

/// Checkpoint in goal execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: String,
    pub description: String,
    pub completed: bool,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Goal orchestration state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalState {
    /// Unique goal identifier.
    pub goal_id: String,
    /// User's objective text.
    pub objective: String,
    /// Current phase.
    pub phase: GoalPhase,
    /// Current status.
    pub status: GoalStatus,
    /// Token budget limit.
    pub token_budget: Option<u64>,
    /// Tokens used so far.
    pub tokens_used: u64,
    /// Budget remaining.
    pub budget_remaining: Option<u64>,
    /// Checkpoints.
    pub checkpoints: Vec<Checkpoint>,
    /// Active subagent role.
    pub active_role: Option<GoalRole>,
    /// Subagent session IDs.
    pub subagent_sessions: HashMap<GoalRole, String>,
    /// Total worker rounds.
    pub worker_rounds: u32,
    /// Total verification rounds.
    pub verify_rounds: u32,
    /// Rounds since last verification.
    pub rounds_since_verify: u32,
    /// Verifier runs attempted.
    pub verifier_attempts: u32,
    /// Created timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
    /// Active since (wall-clock timer).
    pub active_since: Option<DateTime<Utc>>,
    /// Pause reason/message.
    pub pause_message: Option<String>,
}

impl GoalState {
    /// Create a new goal.
    pub fn new(goal_id: String, objective: String, token_budget: Option<u64>) -> Self {
        let now = Utc::now();
        Self {
            goal_id,
            objective,
            phase: GoalPhase::Planning,
            status: GoalStatus::Active,
            token_budget,
            tokens_used: 0,
            budget_remaining: token_budget,
            checkpoints: Vec::new(),
            active_role: Some(GoalRole::Planner),
            subagent_sessions: HashMap::new(),
            worker_rounds: 0,
            verify_rounds: 0,
            rounds_since_verify: 0,
            verifier_attempts: 0,
            created_at: now,
            updated_at: now,
            active_since: Some(Utc::now()),
            pause_message: None,
        }
    }

    /// Add a checkpoint.
    pub fn add_checkpoint(&mut self, id: impl Into<String>, description: impl Into<String>) {
        self.checkpoints.push(Checkpoint {
            id: id.into(),
            description: description.into(),
            completed: false,
            created_at: Utc::now(),
            completed_at: None,
        });
    }

    /// Complete a checkpoint.
    pub fn complete_checkpoint(&mut self, id: &str) {
        if let Some(cp) = self.checkpoints.iter_mut().find(|c| c.id == id) {
            cp.completed = true;
            cp.completed_at = Some(Utc::now());
        }
    }

    /// Record token usage.
    pub fn record_tokens(&mut self, tokens: u64) {
        self.tokens_used += tokens;
        if let Some(budget) = self.budget_remaining {
            self.budget_remaining = Some(budget.saturating_sub(tokens));
        }
    }

    /// Check if budget is exhausted.
    pub fn is_budget_exhausted(&self) -> bool {
        self.budget_remaining == Some(0)
    }
}

/// Goal tracker - state machine for goal execution.
#[derive(Debug)]
pub struct GoalTracker {
    state: RwLock<Option<GoalState>>,
    notify: Arc<Notify>,
}

impl Default for GoalTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl GoalTracker {
    /// Create a new goal tracker.
    pub fn new() -> Self {
        Self {
            state: RwLock::new(None),
            notify: Arc::new(Notify::new()),
        }
    }

    /// Create a new goal.
    pub async fn create_goal(
        &self,
        objective: String,
        token_budget: Option<u64>,
    ) -> GoalState {
        let goal_id = uuid::Uuid::new_v4().to_string();
        let mut goal = GoalState::new(goal_id, objective, token_budget);

        // Initialize checkpoints
        goal.add_checkpoint("plan", "Create implementation plan");
        goal.add_checkpoint("implement", "Implement solution");
        goal.add_checkpoint("verify", "Verify correctness");

        *self.state.write().await = Some(goal.clone());
        self.notify.notify_waiters();

        info!("Created goal: {}", goal.goal_id);
        goal
    }

    /// Get current goal state.
    pub async fn get_state(&self) -> Option<GoalState> {
        self.state.read().await.clone()
    }

    /// Check if a goal is active.
    pub async fn is_active(&self) -> bool {
        if let Some(ref state) = *self.state.read().await {
            matches!(state.phase, GoalPhase::Planning | GoalPhase::Executing)
        } else {
            false
        }
    }

    /// Set goal phase.
    pub async fn set_phase(&self, phase: GoalPhase) -> Option<GoalState> {
        let mut state = self.state.write().await;
        if let Some(ref mut goal) = *state {
            goal.phase = phase;
            goal.updated_at = Utc::now();

            if matches!(phase, GoalPhase::Executing | GoalPhase::Planning) {
                goal.active_since = Some(Utc::now());
            }

            info!("Goal {} transitioned to phase {:?}", goal.goal_id, phase);
            self.notify.notify_waiters();
            return Some(goal.clone());
        }
        None
    }

    /// Pause the goal.
    pub async fn pause(&self, reason: GoalStatus, message: Option<String>) -> Option<GoalState> {
        let mut state = self.state.write().await;
        if let Some(ref mut goal) = *state {
            if !goal.status.is_paused() {
                goal.status = reason;
                goal.phase = GoalPhase::Paused;
                goal.pause_message = message;
                goal.active_since = None;
                goal.updated_at = Utc::now();

                info!("Goal {} paused: {:?}", goal.goal_id, reason);
                self.notify.notify_waiters();
                return Some(goal.clone());
            }
        }
        None
    }

    /// Resume a paused goal.
    pub async fn resume(&self) -> Option<GoalState> {
        let mut state = self.state.write().await;
        if let Some(ref mut goal) = *state {
            if goal.status.is_paused() {
                goal.status = GoalStatus::Active;
                goal.phase = GoalPhase::Executing;
                goal.pause_message = None;
                goal.verifier_attempts = 0;
                goal.active_since = Some(Utc::now());
                goal.updated_at = Utc::now();

                info!("Goal {} resumed", goal.goal_id);
                self.notify.notify_waiters();
                return Some(goal.clone());
            }
        }
        None
    }

    /// Complete the goal successfully.
    pub async fn complete(&self) -> Option<GoalState> {
        let mut state = self.state.write().await;
        if let Some(ref mut goal) = *state {
            goal.phase = GoalPhase::Completed;
            goal.status = GoalStatus::Complete;
            goal.active_since = None;
            goal.updated_at = Utc::now();

            // Complete all checkpoints
            for cp in &mut goal.checkpoints {
                if !cp.completed {
                    cp.completed = true;
                    cp.completed_at = Some(Utc::now());
                }
            }

            info!("Goal {} completed", goal.goal_id);
            self.notify.notify_waiters();
            return Some(goal.clone());
        }
        None
    }

    /// Mark goal as budget limited.
    pub async fn budget_limit(&self) -> Option<GoalState> {
        let mut state = self.state.write().await;
        if let Some(ref mut goal) = *state {
            goal.phase = GoalPhase::Failed;
            goal.status = GoalStatus::BudgetLimited;
            goal.active_since = None;
            goal.updated_at = Utc::now();

            warn!("Goal {} budget exhausted", goal.goal_id);
            self.notify.notify_waiters();
            return Some(goal.clone());
        }
        None
    }

    /// Fail the goal.
    pub async fn fail(&self, message: impl Into<String>) -> Option<GoalState> {
        let mut state = self.state.write().await;
        if let Some(ref mut goal) = *state {
            goal.phase = GoalPhase::Failed;
            goal.pause_message = Some(message.into());
            goal.active_since = None;
            goal.updated_at = Utc::now();

            warn!("Goal {} failed", goal.goal_id);
            self.notify.notify_waiters();
            return Some(goal.clone());
        }
        None
    }

    /// Clear the goal.
    pub async fn clear(&self) {
        let mut state = self.state.write().await;
        if state.is_some() {
            info!("Cleared goal");
            *state = None;
            self.notify.notify_waiters();
        }
    }

    /// Record worker round completion.
    pub async fn record_worker_round(&self) -> Option<GoalState> {
        let mut state = self.state.write().await;
        if let Some(ref mut goal) = *state {
            goal.worker_rounds += 1;
            goal.rounds_since_verify += 1;
            goal.updated_at = Utc::now();
            goal.active_since = Some(Utc::now());

            debug!(
                "Worker round {} completed ({} since verify)",
                goal.worker_rounds, goal.rounds_since_verify
            );

            return Some(goal.clone());
        }
        None
    }

    /// Record verification round completion.
    pub async fn record_verify_round(&self) -> Option<GoalState> {
        let mut state = self.state.write().await;
        if let Some(ref mut goal) = *state {
            goal.verify_rounds += 1;
            goal.rounds_since_verify = 0;
            goal.verifier_attempts += 1;
            goal.updated_at = Utc::now();

            return Some(goal.clone());
        }
        None
    }

    /// Record subagent session for a role.
    pub async fn set_subagent_session(&self, role: GoalRole, session_id: String) -> Option<GoalState> {
        let mut state = self.state.write().await;
        if let Some(ref mut goal) = *state {
            goal.subagent_sessions.insert(role, session_id);
            goal.active_role = Some(role);
            goal.updated_at = Utc::now();
            return Some(goal.clone());
        }
        None
    }

    /// Update goal progress (for agent reporting).
    pub async fn update_progress(
        &self,
        checkpoint_id: Option<&str>,
        tokens_used: u64,
        message: Option<String>,
    ) -> Option<GoalState> {
        let mut state = self.state.write().await;
        if let Some(ref mut goal) = *state {
            if let Some(id) = checkpoint_id {
                goal.complete_checkpoint(id);
            }
            goal.record_tokens(tokens_used);
            if let Some(msg) = message {
                goal.pause_message = Some(msg);
            }
            goal.updated_at = Utc::now();
            goal.active_since = Some(Utc::now());

            self.notify.notify_waiters();
            return Some(goal.clone());
        }
        None
    }

    /// Wait for goal state change.
    pub async fn wait(&self) {
        self.notify.notified().await;
    }

    /// Wait with timeout.
    pub async fn wait_timeout(&self, timeout: Duration) -> bool {
        tokio::time::timeout(timeout, self.notify.notified())
            .await
            .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn create_and_complete_goal() {
        let tracker = GoalTracker::new();

        let goal = tracker.create_goal("Implement feature X".to_string(), Some(10000)).await;
        assert_eq!(goal.phase, GoalPhase::Planning);
        assert_eq!(goal.objective, "Implement feature X");
        assert!(goal.checkpoints.len() == 3);

        // Transition through phases
        tracker.set_phase(GoalPhase::Executing).await;
        let state = tracker.get_state().await.unwrap();
        assert_eq!(state.phase, GoalPhase::Executing);

        tracker.complete().await;
        let state = tracker.get_state().await.unwrap();
        assert_eq!(state.phase, GoalPhase::Completed);
    }

    #[tokio::test]
    async fn pause_and_resume() {
        let tracker = GoalTracker::new();

        tracker.create_goal("Test goal".to_string(), None).await;
        tracker.set_phase(GoalPhase::Executing).await;

        tracker.pause(GoalStatus::UserPaused, Some("User requested".to_string())).await;
        let state = tracker.get_state().await.unwrap();
        assert!(state.status.is_paused());

        tracker.resume().await;
        let state = tracker.get_state().await.unwrap();
        assert!(!state.status.is_paused());
    }

    #[tokio::test]
    async fn token_tracking() {
        let tracker = GoalTracker::new();

        tracker.create_goal("Test".to_string(), Some(1000)).await;
        tracker.update_progress(None, 100, None).await;

        let state = tracker.get_state().await.unwrap();
        assert_eq!(state.tokens_used, 100);
        assert_eq!(state.budget_remaining, Some(900));
    }

    #[tokio::test]
    async fn checkpoint_completion() {
        let tracker = GoalTracker::new();

        tracker.create_goal("Test".to_string(), None).await;
        tracker.update_progress(Some("plan"), 0, None).await;

        let state = tracker.get_state().await.unwrap();
        let plan = state.checkpoints.iter().find(|c| c.id == "plan").unwrap();
        assert!(plan.completed);
    }
}
