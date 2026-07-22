//! Unit tests for GoalTracker state machine.

use runie_core::goal::{GoalPhase, GoalRole, GoalState, GoalStatus, GoalTracker};

#[tokio::test]
async fn create_goal_initializes_state() {
    let tracker = GoalTracker::new();
    let goal = tracker.create_goal("Test objective".to_string(), Some(10000)).await;

    assert_eq!(goal.objective, "Test objective");
    assert_eq!(goal.phase, GoalPhase::Planning);
    assert_eq!(goal.status, GoalStatus::Active);
    assert_eq!(goal.token_budget, Some(10000));
    assert_eq!(goal.budget_remaining, Some(10000));
    assert!(!goal.checkpoints.is_empty());
}

#[tokio::test]
async fn create_goal_generates_checkpoints() {
    let tracker = GoalTracker::new();
    let goal = tracker.create_goal("Test".to_string(), None).await;

    // Should have plan, implement, verify checkpoints
    assert!(goal.checkpoints.iter().any(|c| c.id == "plan"));
    assert!(goal.checkpoints.iter().any(|c| c.id == "implement"));
    assert!(goal.checkpoints.iter().any(|c| c.id == "verify"));
}

#[tokio::test]
async fn set_phase_updates_phase() {
    let tracker = GoalTracker::new();
    tracker.create_goal("Test".to_string(), None).await;

    tracker.set_phase(GoalPhase::Executing).await;
    let state = tracker.get_state().await.unwrap();
    assert_eq!(state.phase, GoalPhase::Executing);
}

#[tokio::test]
async fn pause_sets_status_and_phase() {
    let tracker = GoalTracker::new();
    tracker.create_goal("Test".to_string(), None).await;

    tracker.pause(GoalStatus::UserPaused, Some("User requested".to_string())).await;
    let state = tracker.get_state().await.unwrap();

    assert!(state.status.is_paused());
    assert_eq!(state.phase, GoalPhase::Paused);
    assert_eq!(state.pause_message, Some("User requested".to_string()));
}

#[tokio::test]
async fn resume_clears_paused_state() {
    let tracker = GoalTracker::new();
    tracker.create_goal("Test".to_string(), None).await;

    tracker.pause(GoalStatus::UserPaused, None).await;
    tracker.resume().await;
    let state = tracker.get_state().await.unwrap();

    assert_eq!(state.status, GoalStatus::Active);
    assert_eq!(state.phase, GoalPhase::Executing);
    assert!(state.pause_message.is_none());
}

#[tokio::test]
async fn complete_marks_all_checkpoints_done() {
    let tracker = GoalTracker::new();
    tracker.create_goal("Test".to_string(), None).await;

    tracker.complete().await;
    let state = tracker.get_state().await.unwrap();

    assert_eq!(state.phase, GoalPhase::Completed);
    assert_eq!(state.status, GoalStatus::Complete);
    assert!(state.checkpoints.iter().all(|c| c.completed));
}

#[tokio::test]
async fn fail_sets_failed_phase() {
    let tracker = GoalTracker::new();
    tracker.create_goal("Test".to_string(), None).await;

    tracker.fail("Implementation blocked").await;
    let state = tracker.get_state().await.unwrap();

    assert_eq!(state.phase, GoalPhase::Failed);
    assert_eq!(state.pause_message, Some("Implementation blocked".to_string()));
}

#[tokio::test]
async fn budget_limit_sets_status() {
    let tracker = GoalTracker::new();
    tracker.create_goal("Test".to_string(), Some(1000)).await;

    tracker.budget_limit().await;
    let state = tracker.get_state().await.unwrap();

    assert_eq!(state.phase, GoalPhase::Failed);
    assert_eq!(state.status, GoalStatus::BudgetLimited);
}

#[tokio::test]
async fn clear_removes_goal() {
    let tracker = GoalTracker::new();
    tracker.create_goal("Test".to_string(), None).await;

    tracker.clear().await;
    let state = tracker.get_state().await;

    assert!(state.is_none());
}

#[tokio::test]
async fn is_active_returns_correct_state() {
    let tracker = GoalTracker::new();

    // No goal
    assert!(!tracker.is_active().await);

    // Planning phase
    tracker.create_goal("Test".to_string(), None).await;
    assert!(tracker.is_active().await);

    // Paused
    tracker.pause(GoalStatus::UserPaused, None).await;
    assert!(!tracker.is_active().await);

    // Resume
    tracker.resume().await;
    assert!(tracker.is_active().await);
}

#[tokio::test]
async fn record_worker_round_increments_counter() {
    let tracker = GoalTracker::new();
    tracker.create_goal("Test".to_string(), None).await;

    tracker.record_worker_round().await;
    let state = tracker.get_state().await.unwrap();
    assert_eq!(state.worker_rounds, 1);

    tracker.record_worker_round().await;
    let state = tracker.get_state().await.unwrap();
    assert_eq!(state.worker_rounds, 2);
}

#[tokio::test]
async fn record_verify_round_resets_rounds_since_verify() {
    let tracker = GoalTracker::new();
    tracker.create_goal("Test".to_string(), None).await;

    tracker.record_worker_round().await;
    tracker.record_worker_round().await;
    {
        let state = tracker.get_state().await.unwrap();
        assert_eq!(state.rounds_since_verify, 2);
    }

    tracker.record_verify_round().await;
    let state = tracker.get_state().await.unwrap();
    assert_eq!(state.rounds_since_verify, 0);
    assert_eq!(state.verify_rounds, 1);
}

#[tokio::test]
async fn set_subagent_session_tracks_session() {
    let tracker = GoalTracker::new();
    tracker.create_goal("Test".to_string(), None).await;

    tracker.set_subagent_session(GoalRole::Worker, "session-123".to_string()).await;
    let state = tracker.get_state().await.unwrap();

    assert_eq!(state.subagent_sessions.get(&GoalRole::Worker), Some(&"session-123".to_string()));
    assert_eq!(state.active_role, Some(GoalRole::Worker));
}

#[tokio::test]
async fn update_progress_completes_checkpoint_and_records_tokens() {
    let tracker = GoalTracker::new();
    tracker.create_goal("Test".to_string(), Some(1000)).await;

    tracker.update_progress(Some("plan"), 100, None).await;
    let state = tracker.get_state().await.unwrap();

    assert_eq!(state.tokens_used, 100);
    assert_eq!(state.budget_remaining, Some(900));

    let plan = state.checkpoints.iter().find(|c| c.id == "plan").unwrap();
    assert!(plan.completed);
    assert!(plan.completed_at.is_some());
}

#[tokio::test]
async fn token_tracking_works() {
    let tracker = GoalTracker::new();
    tracker.create_goal("Test".to_string(), Some(1000)).await;

    tracker.update_progress(None, 250, None).await;
    let state = tracker.get_state().await.unwrap();
    assert_eq!(state.tokens_used, 250);
    assert_eq!(state.budget_remaining, Some(750));

    tracker.update_progress(None, 300, None).await;
    let state = tracker.get_state().await.unwrap();
    assert_eq!(state.tokens_used, 550);
    assert_eq!(state.budget_remaining, Some(450));
}

#[tokio::test]
async fn budget_exhaustion_detected() {
    let tracker = GoalTracker::new();
    tracker.create_goal("Test".to_string(), Some(100)).await;

    // Consume all budget
    tracker.update_progress(None, 100, None).await;
    let state = tracker.get_state().await.unwrap();

    assert!(state.is_budget_exhausted());
    assert_eq!(state.budget_remaining, Some(0));
}

#[tokio::test]
async fn pause_idempotent_when_already_paused() {
    let tracker = GoalTracker::new();
    tracker.create_goal("Test".to_string(), None).await;
    tracker.pause(GoalStatus::UserPaused, None).await;

    // Try to pause again with different reason
    tracker.pause(GoalStatus::BackOffPaused, None).await;
    let state = tracker.get_state().await.unwrap();

    // Status should remain UserPaused (first pause wins)
    assert_eq!(state.status, GoalStatus::UserPaused);
}

#[tokio::test]
async fn resume_idempotent_when_not_paused() {
    let tracker = GoalTracker::new();
    tracker.create_goal("Test".to_string(), None).await;

    // Try to resume when not paused
    tracker.resume().await;
    let state = tracker.get_state().await.unwrap();

    // Should remain active
    assert_eq!(state.status, GoalStatus::Active);
}

#[tokio::test]
async fn goal_state_serialization_round_trip() {
    let tracker = GoalTracker::new();
    let original = tracker.create_goal("Serialize test".to_string(), Some(5000)).await;

    let json = serde_json::to_string(&original).unwrap();
    let restored: GoalState = serde_json::from_str(&json).unwrap();

    assert_eq!(restored.goal_id, original.goal_id);
    assert_eq!(restored.objective, original.objective);
    assert_eq!(restored.phase, original.phase);
    assert_eq!(restored.status, original.status);
    assert_eq!(restored.checkpoints.len(), original.checkpoints.len());
}
