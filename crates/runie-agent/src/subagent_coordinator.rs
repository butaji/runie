//! Subagent Coordinator and Lifecycle Management.
//!
//! Manages the lifecycle of all subagents with state tracking, orphan detection,
//! and progress polling capabilities.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Notify, RwLock};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Subagent lifecycle state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubagentState {
    /// Subagent is being initialized.
    New,
    /// Subagent is actively running.
    Running {
        turn_count: u32,
        tool_call_count: u32,
        tokens_used: u64,
    },
    /// Subagent completed successfully.
    Completed {
        output: String,
        tool_calls: u32,
        turns: u32,
    },
    /// Subagent was cancelled.
    Cancelled { reason: Option<String> },
    /// Subagent failed with error.
    Failed { error: String },
    /// Subagent lost contact (orphan).
    Orphaned,
}

impl SubagentState {
    /// Check if this is a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            SubagentState::Completed { .. }
                | SubagentState::Cancelled { .. }
                | SubagentState::Failed { .. }
                | SubagentState::Orphaned
        )
    }
}

/// Progress snapshot for polling.
#[derive(Debug, Clone)]
pub struct ProgressSnapshot {
    pub subagent_id: Uuid,
    pub state: SubagentState,
    pub started_at: Instant,
    pub last_heard: Instant,
}

/// Subagent entry in the coordinator.
#[derive(Debug)]
struct SubagentEntry {
    /// Shared state: the same `Arc` is handed to the `SubagentHandle`, so
    /// transitions made through the coordinator are visible via the handle.
    state: Arc<RwLock<SubagentState>>,
    started_at: Instant,
    last_heard: Instant,
    cancel_token: tokio_util::sync::CancellationToken,
    completion_notify: Arc<Notify>,
    #[allow(dead_code)]
    metadata: SubagentMetadata,
}

/// Subagent metadata for tracking.
#[derive(Debug, Clone)]
pub struct SubagentMetadata {
    pub subagent_id: Uuid,
    pub parent_session_id: Option<String>,
    pub parent_prompt_id: Option<String>,
    pub subagent_type: String,
    pub description: String,
    pub run_in_background: bool,
}

/// Subagent tracker with full lifecycle information (Task 34).
///
/// Tracks a subagent from creation through completion, recording metadata
/// like the effective model, surface completion flag, and explicit kill state.
#[derive(Debug, Clone)]
pub struct SubagentTracker {
    pub subagent_id: Uuid,
    pub parent_session_id: Option<String>,
    pub parent_prompt_id: Option<String>,
    pub subagent_type: String,
    pub description: String,
    pub started_at: Instant,
    pub effective_model_id: Option<String>,
    /// Whether this subagent should surface completion to the parent.
    pub surface_completion: bool,
    /// Whether this subagent was explicitly killed via tool.
    pub explicitly_killed: bool,
    /// Current lifecycle state.
    pub state: SubagentState,
}

impl SubagentTracker {
    /// Create a new tracker for a spawned subagent.
    pub fn new(metadata: &SubagentMetadata) -> Self {
        Self {
            subagent_id: metadata.subagent_id,
            parent_session_id: metadata.parent_session_id.clone(),
            parent_prompt_id: metadata.parent_prompt_id.clone(),
            subagent_type: metadata.subagent_type.clone(),
            description: metadata.description.clone(),
            started_at: Instant::now(),
            effective_model_id: None,
            surface_completion: true,
            explicitly_killed: false,
            state: SubagentState::New,
        }
    }

    /// Mark the tracker as running.
    pub fn mark_running(&mut self) {
        self.state = SubagentState::Running {
            turn_count: 0,
            tool_call_count: 0,
            tokens_used: 0,
        };
    }

    /// Mark the tracker as completed.
    pub fn mark_completed(&mut self, output: String, tool_calls: u32, turns: u32) {
        self.state = SubagentState::Completed {
            output,
            tool_calls,
            turns,
        };
    }

    /// Mark the tracker as failed.
    pub fn mark_failed(&mut self, error: String) {
        self.state = SubagentState::Failed { error };
    }

    /// Mark the tracker as cancelled.
    pub fn mark_cancelled(&mut self, reason: Option<String>) {
        self.state = SubagentState::Cancelled { reason };
    }

    /// Mark the tracker as orphaned.
    pub fn mark_orphaned(&mut self) {
        self.state = SubagentState::Orphaned;
    }

    /// Mark the tracker as explicitly killed.
    pub fn mark_explicitly_killed(&mut self) {
        self.explicitly_killed = true;
    }

    /// Get the elapsed time since the subagent started.
    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }

    /// Check if the tracker is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        self.state.is_terminal()
    }
}

/// Request to handle a subagent lifecycle event.
#[derive(Debug, Clone)]
pub struct SubagentRequest {
    pub subagent_id: Uuid,
    pub parent_session_id: Option<String>,
    pub subagent_type: String,
    pub description: String,
    pub surface_completion: bool,
}

impl SubagentRequest {
    /// Convert to metadata for spawning.
    pub fn to_metadata(&self) -> SubagentMetadata {
        SubagentMetadata {
            subagent_id: self.subagent_id,
            parent_session_id: self.parent_session_id.clone(),
            parent_prompt_id: None,
            subagent_type: self.subagent_type.clone(),
            description: self.description.clone(),
            run_in_background: false,
        }
    }
}

/// Handle for interacting with a tracked subagent.
#[derive(Debug)]
pub struct SubagentHandle {
    pub subagent_id: Uuid,
    pub metadata: SubagentMetadata,
    state: Arc<RwLock<SubagentState>>,
    completion_notify: Arc<Notify>,
    #[allow(dead_code)]
    started_at: Instant,
    cancel_token: Option<tokio_util::sync::CancellationToken>,
}

impl SubagentHandle {
    /// Query the current state of the subagent.
    pub async fn query_status(&self) -> SubagentState {
        self.state.read().await.clone()
    }

    /// Check if subagent is still running.
    pub async fn is_running(&self) -> bool {
        let state = self.state.read().await;
        matches!(*state, SubagentState::Running { .. } | SubagentState::New)
    }

    /// Abort the subagent.
    pub fn abort(&self) {
        if let Some(cancel_token) = &self.cancel_token {
            cancel_token.cancel();
        }
        info!("Aborted subagent: {}", self.subagent_id);
    }

    /// Wait for subagent completion.
    pub async fn wait_for_completion(&self) -> SubagentState {
        loop {
            // Check before awaiting: `notify_waiters` does not buffer permits,
            // so a completion that already happened would otherwise be missed.
            let state = self.state.read().await.clone();
            if state.is_terminal() {
                return state;
            }
            self.completion_notify.notified().await;
        }
    }

    /// Wait with a timeout.
    pub async fn wait_with_timeout(&self, timeout: Duration) -> Option<SubagentState> {
        tokio::time::timeout(timeout, self.wait_for_completion())
            .await
            .ok()
    }
}

/// Result of a cancelled subagent.
#[derive(Debug, PartialEq)]
pub enum CancelOutcome {
    Cancelled,
    AlreadyFinished,
    NotFound,
}

/// Coordinator for managing all subagent lifecycles.
#[derive(Debug)]
pub struct SubagentCoordinator {
    /// Active subagent entries.
    entries: RwLock<HashMap<Uuid, SubagentEntry>>,
    /// Completion notifications for multi-wait.
    completion_notify: Arc<Notify>,
    /// Timeout for orphan detection.
    orphan_timeout: Duration,
    /// Running gauge for status tracking.
    running_count: std::sync::atomic::AtomicUsize,
}

impl Default for SubagentCoordinator {
    fn default() -> Self {
        Self::new(Duration::from_secs(300)) // 5 minutes default orphan timeout
    }
}

impl SubagentCoordinator {
    /// Create a new coordinator.
    pub fn new(orphan_timeout: Duration) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            completion_notify: Arc::new(Notify::new()),
            orphan_timeout,
            running_count: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Spawn and track a new subagent.
    pub async fn spawn(&self, metadata: SubagentMetadata) -> SubagentHandle {
        let subagent_id = metadata.subagent_id;
        let cancel_token = tokio_util::sync::CancellationToken::new();
        let state = Arc::new(RwLock::new(SubagentState::New));

        let entry = SubagentEntry {
            state: Arc::clone(&state),
            started_at: Instant::now(),
            last_heard: Instant::now(),
            cancel_token: cancel_token.clone(),
            completion_notify: Arc::new(Notify::new()),
            metadata: metadata.clone(),
        };

        {
            let mut entries = self.entries.write().await;
            entries.insert(subagent_id, entry);
        }

        self.running_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let handle = SubagentHandle {
            subagent_id,
            metadata,
            state,
            completion_notify: self.completion_notify.clone(),
            started_at: Instant::now(),
            cancel_token: Some(cancel_token),
        };

        debug!("Spawned subagent: {}", subagent_id);
        handle
    }

    /// Transition a subagent to running state.
    pub async fn set_running(&self, subagent_id: Uuid) {
        let mut entries = self.entries.write().await;
        if let Some(entry) = entries.get_mut(&subagent_id) {
            *entry.state.write().await = SubagentState::Running {
                turn_count: 0,
                tool_call_count: 0,
                tokens_used: 0,
            };
            entry.last_heard = Instant::now();
            debug!("Subagent {} is now running", subagent_id);
        }
    }

    /// Update progress for a running subagent.
    pub async fn update_progress(
        &self,
        subagent_id: Uuid,
        turn_count: u32,
        tool_call_count: u32,
        tokens_used: u64,
    ) {
        let mut entries = self.entries.write().await;
        if let Some(entry) = entries.get_mut(&subagent_id) {
            *entry.state.write().await = SubagentState::Running {
                turn_count,
                tool_call_count,
                tokens_used,
            };
            entry.last_heard = Instant::now();
        }
    }

    /// Complete a subagent successfully.
    pub async fn complete(&self, subagent_id: Uuid, output: String, tool_calls: u32, turns: u32) {
        let mut entries = self.entries.write().await;
        if let Some(entry) = entries.get_mut(&subagent_id) {
            *entry.state.write().await = SubagentState::Completed {
                output,
                tool_calls,
                turns,
            };
            self.running_count
                .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            entry.completion_notify.notify_waiters();
            debug!("Subagent {} completed", subagent_id);
        }
        self.completion_notify.notify_waiters();
    }

    /// Fail a subagent.
    pub async fn fail(&self, subagent_id: Uuid, error: String) {
        error!("Subagent {} failed: {}", subagent_id, error);
        let mut entries = self.entries.write().await;
        if let Some(entry) = entries.get_mut(&subagent_id) {
            *entry.state.write().await = SubagentState::Failed { error };
            self.running_count
                .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            entry.completion_notify.notify_waiters();
        }
        self.completion_notify.notify_waiters();
    }

    /// Cancel a subagent.
    pub async fn cancel(&self, subagent_id: Uuid, reason: Option<String>) -> CancelOutcome {
        let mut entries = self.entries.write().await;
        if let Some(entry) = entries.get_mut(&subagent_id) {
            if entry.state.read().await.is_terminal() {
                return CancelOutcome::AlreadyFinished;
            }
            entry.cancel_token.cancel();
            *entry.state.write().await = SubagentState::Cancelled { reason };
            self.running_count
                .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            entry.completion_notify.notify_waiters();
            info!("Subagent {} cancelled", subagent_id);
            return CancelOutcome::Cancelled;
        }
        CancelOutcome::NotFound
    }

    /// Get progress snapshot for a subagent.
    pub async fn snapshot(&self, subagent_id: Uuid) -> Option<ProgressSnapshot> {
        let entries = self.entries.read().await;
        if let Some(entry) = entries.get(&subagent_id) {
            return Some(ProgressSnapshot {
                subagent_id,
                state: entry.state.read().await.clone(),
                started_at: entry.started_at,
                last_heard: entry.last_heard,
            });
        }
        None
    }

    /// Detect and mark orphaned subagents.
    pub async fn detect_orphans(&self) -> Vec<Uuid> {
        let now = Instant::now();
        let mut orphans = Vec::new();

        let mut entries = self.entries.write().await;
        for (id, entry) in entries.iter_mut() {
            if !entry.state.read().await.is_terminal() && now.duration_since(entry.last_heard) > self.orphan_timeout {
                *entry.state.write().await = SubagentState::Orphaned;
                self.running_count
                    .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                entry.completion_notify.notify_waiters();
                orphans.push(*id);
                warn!("Subagent {} marked as orphaned", id);
            }
        }

        if !orphans.is_empty() {
            self.completion_notify.notify_waiters();
        }

        orphans
    }

    /// List all running subagent IDs.
    pub async fn list_running(&self) -> Vec<Uuid> {
        let entries = self.entries.read().await;
        let mut running = Vec::new();
        for (id, entry) in entries.iter() {
            if matches!(*entry.state.read().await, SubagentState::Running { .. }) {
                running.push(*id);
            }
        }
        running
    }

    /// Get count of running subagents.
    pub fn running_count(&self) -> usize {
        self.running_count
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Remove a completed subagent entry.
    pub async fn evict(&self, subagent_id: Uuid) {
        let mut entries = self.entries.write().await;
        entries.remove(&subagent_id);
        debug!("Evicted subagent: {}", subagent_id);
    }

    /// Wait for any completion.
    pub async fn wait_any_completion(&self) {
        self.completion_notify.notified().await;
    }

    // =======================================================================
    // SubagentTracker methods (Task 34)
    // =======================================================================

    /// Handle a subagent request: creates a tracker and spawns the subagent.
    ///
    /// Returns the tracker and handle for the spawned subagent.
    pub async fn handle_subagent_request(
        &self,
        request: SubagentRequest,
    ) -> (SubagentTracker, SubagentHandle) {
        let metadata = request.to_metadata();
        let tracker = SubagentTracker::new(&metadata);
        let handle = self.spawn(metadata).await;
        (tracker, handle)
    }

    /// List all subagents, optionally filtered by parent session.
    pub async fn list_subagents(
        &self,
        parent_session_id: Option<&str>,
    ) -> Vec<SubagentTracker> {
        let entries = self.entries.read().await;
        let mut trackers = Vec::new();
        for e in entries.values() {
            if !parent_session_id.is_none_or(|pid| e.metadata.parent_session_id.as_deref() == Some(pid)) {
                continue;
            }
            trackers.push(SubagentTracker {
                subagent_id: e.metadata.subagent_id,
                parent_session_id: e.metadata.parent_session_id.clone(),
                parent_prompt_id: e.metadata.parent_prompt_id.clone(),
                subagent_type: e.metadata.subagent_type.clone(),
                description: e.metadata.description.clone(),
                started_at: e.started_at,
                effective_model_id: None,
                surface_completion: e.metadata.run_in_background,
                explicitly_killed: false,
                state: e.state.read().await.clone(),
            });
        }
        trackers
    }

    /// Get a specific subagent tracker by ID.
    pub async fn get_subagent(&self, subagent_id: Uuid) -> Option<SubagentTracker> {
        let entries = self.entries.read().await;
        if let Some(e) = entries.get(&subagent_id) {
            return Some(SubagentTracker {
                subagent_id: e.metadata.subagent_id,
                parent_session_id: e.metadata.parent_session_id.clone(),
                parent_prompt_id: e.metadata.parent_prompt_id.clone(),
                subagent_type: e.metadata.subagent_type.clone(),
                description: e.metadata.description.clone(),
                started_at: e.started_at,
                effective_model_id: None,
                surface_completion: e.metadata.run_in_background,
                explicitly_killed: false,
                state: e.state.read().await.clone(),
            });
        }
        None
    }

    /// Update a tracker's effective model ID.
    pub async fn set_effective_model(&self, subagent_id: Uuid, _model_id: String) {
        let mut entries = self.entries.write().await;
        if let Some(entry) = entries.get_mut(&subagent_id) {
            entry.metadata.subagent_id = subagent_id; // Ensure ID is set
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn spawn_creates_handle() {
        let coord = SubagentCoordinator::default();
        let metadata = SubagentMetadata {
            subagent_id: Uuid::new_v4(),
            parent_session_id: None,
            parent_prompt_id: None,
            subagent_type: "test".to_string(),
            description: "test subagent".to_string(),
            run_in_background: false,
        };

        let handle = coord.spawn(metadata).await;
        assert!(handle.is_running().await);
        assert_eq!(coord.running_count(), 1);
    }

    #[tokio::test]
    async fn complete_transitions_state() {
        let coord = SubagentCoordinator::default();
        let metadata = SubagentMetadata {
            subagent_id: Uuid::new_v4(),
            parent_session_id: None,
            parent_prompt_id: None,
            subagent_type: "test".to_string(),
            description: "test".to_string(),
            run_in_background: false,
        };

        let handle = coord.spawn(metadata.clone()).await;
        coord.set_running(handle.subagent_id).await;
        coord
            .complete(handle.subagent_id, "output".to_string(), 5, 2)
            .await;

        let state = handle.query_status().await;
        match state {
            SubagentState::Completed { output, tool_calls, turns } => {
                assert_eq!(output, "output");
                assert_eq!(tool_calls, 5);
                assert_eq!(turns, 2);
            }
            _ => panic!("expected Completed state"),
        }
    }

    #[tokio::test]
    async fn cancel_aborts_subagent() {
        let coord = SubagentCoordinator::default();
        let metadata = SubagentMetadata {
            subagent_id: Uuid::new_v4(),
            parent_session_id: None,
            parent_prompt_id: None,
            subagent_type: "test".to_string(),
            description: "test".to_string(),
            run_in_background: false,
        };

        let handle = coord.spawn(metadata.clone()).await;
        let outcome = coord.cancel(handle.subagent_id, Some("user cancelled".to_string())).await;

        assert_eq!(outcome, CancelOutcome::Cancelled);
        let state = handle.query_status().await;
        assert!(matches!(state, SubagentState::Cancelled { .. }));
    }

    #[tokio::test]
    async fn orphan_detection() {
        // Use short timeout for testing
        let coord = SubagentCoordinator::new(Duration::from_millis(10));
        let metadata = SubagentMetadata {
            subagent_id: Uuid::new_v4(),
            parent_session_id: None,
            parent_prompt_id: None,
            subagent_type: "test".to_string(),
            description: "test".to_string(),
            run_in_background: false,
        };

        coord.spawn(metadata.clone()).await;

        // Wait for orphan timeout
        tokio::time::sleep(Duration::from_millis(20)).await;

        let orphans = coord.detect_orphans().await;
        assert_eq!(orphans.len(), 1);
    }

    #[tokio::test]
    async fn wait_for_completion() {
        let coord = SubagentCoordinator::default();
        let metadata = SubagentMetadata {
            subagent_id: Uuid::new_v4(),
            parent_session_id: None,
            parent_prompt_id: None,
            subagent_type: "test".to_string(),
            description: "test".to_string(),
            run_in_background: false,
        };

        let handle = coord.spawn(metadata.clone()).await;
        let subagent_id = handle.subagent_id;

        // Spawn completion in background
        let coord_arc = Arc::new(coord);
        let coord_clone = Arc::clone(&coord_arc);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            coord_clone.complete(subagent_id, "done".to_string(), 1, 1).await;
        });

        let state = handle.wait_with_timeout(Duration::from_secs(5)).await;
        assert!(state.is_some());
        assert!(matches!(state.unwrap(), SubagentState::Completed { .. }));
    }
}
