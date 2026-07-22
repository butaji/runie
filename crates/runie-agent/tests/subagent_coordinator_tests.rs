//! Tests for subagent coordinator and lifecycle management (Task 34).
//!
//! These tests verify the SubagentCoordinator correctly tracks subagent
//! lifecycles and the SubagentTracker provides proper metadata.

use runie_agent::{
    SubagentCoordinator, SubagentMetadata, SubagentRequest, SubagentState,
};
use std::time::Duration;
use uuid::Uuid;

/// Test: SubagentTracker tracks metadata correctly
#[test]
fn tracker_tracks_metadata() {
    let metadata = SubagentMetadata {
        subagent_id: Uuid::new_v4(),
        parent_session_id: Some("session-1".to_string()),
        parent_prompt_id: Some("prompt-1".to_string()),
        subagent_type: "worker".to_string(),
        description: "test worker".to_string(),
        run_in_background: false,
    };
    
    let tracker = runie_agent::SubagentTracker::new(&metadata);
    
    assert_eq!(tracker.subagent_id, metadata.subagent_id);
    assert_eq!(tracker.parent_session_id, Some("session-1".to_string()));
    assert_eq!(tracker.subagent_type, "worker");
    assert_eq!(tracker.state, SubagentState::New);
    assert!(!tracker.explicitly_killed);
}

/// Test: Tracker state transitions
#[test]
fn tracker_state_transitions() {
    let metadata = SubagentMetadata {
        subagent_id: Uuid::new_v4(),
        parent_session_id: None,
        parent_prompt_id: None,
        subagent_type: "test".to_string(),
        description: "test".to_string(),
        run_in_background: false,
    };
    
    let mut tracker = runie_agent::SubagentTracker::new(&metadata);
    
    // New -> Running
    tracker.mark_running();
    assert!(matches!(tracker.state, SubagentState::Running { .. }));
    
    // Running -> Completed
    tracker.mark_completed("output".to_string(), 5, 2);
    assert!(matches!(tracker.state, SubagentState::Completed { ref output, .. } if *output == "output"));
    assert!(matches!(tracker.state, SubagentState::Completed { .. }));
}

/// Test: Tracker marks explicit kill
#[test]
fn tracker_marks_explicit_kill() {
    let metadata = SubagentMetadata {
        subagent_id: Uuid::new_v4(),
        parent_session_id: None,
        parent_prompt_id: None,
        subagent_type: "test".to_string(),
        description: "test".to_string(),
        run_in_background: false,
    };
    
    let mut tracker = runie_agent::SubagentTracker::new(&metadata);
    
    tracker.mark_explicitly_killed();
    assert!(tracker.explicitly_killed);
}

/// Test: SubagentRequest converts to metadata
#[test]
fn request_to_metadata() {
    let request = SubagentRequest {
        subagent_id: Uuid::new_v4(),
        parent_session_id: Some("parent-session".to_string()),
        subagent_type: "planner".to_string(),
        description: "plan the work".to_string(),
        surface_completion: true,
    };
    
    let metadata = request.to_metadata();
    
    assert_eq!(metadata.subagent_id, request.subagent_id);
    assert_eq!(metadata.parent_session_id, request.parent_session_id);
    assert_eq!(metadata.subagent_type, request.subagent_type);
    assert_eq!(metadata.description, request.description);
}

/// Test: handle_subagent_request creates tracker and handle
#[tokio::test]
async fn handle_subagent_request() {
    let coordinator = SubagentCoordinator::default();
    
    let request = SubagentRequest {
        subagent_id: Uuid::new_v4(),
        parent_session_id: Some("session-1".to_string()),
        subagent_type: "worker".to_string(),
        description: "do the work".to_string(),
        surface_completion: true,
    };
    
    let (tracker, handle) = coordinator.handle_subagent_request(request).await;
    
    assert_eq!(tracker.subagent_type, "worker");
    assert_eq!(tracker.parent_session_id, Some("session-1".to_string()));
    assert_eq!(handle.subagent_id, tracker.subagent_id);
}

/// Test: list_subagents filters by parent session
#[tokio::test]
async fn list_subagents_filters_by_session() {
    let coordinator = SubagentCoordinator::default();
    
    // Spawn workers in different sessions
    let meta1 = SubagentMetadata {
        subagent_id: Uuid::new_v4(),
        parent_session_id: Some("session-A".to_string()),
        parent_prompt_id: None,
        subagent_type: "worker".to_string(),
        description: "task 1".to_string(),
        run_in_background: false,
    };
    
    let meta2 = SubagentMetadata {
        subagent_id: Uuid::new_v4(),
        parent_session_id: Some("session-B".to_string()),
        parent_prompt_id: None,
        subagent_type: "worker".to_string(),
        description: "task 2".to_string(),
        run_in_background: false,
    };
    
    coordinator.spawn(meta1).await;
    coordinator.spawn(meta2).await;
    
    // List workers for session-A
    let session_a_workers = coordinator.list_subagents(Some("session-A")).await;
    assert_eq!(session_a_workers.len(), 1);
    assert_eq!(session_a_workers[0].parent_session_id, Some("session-A".to_string()));
    
    // List all workers
    let all_workers = coordinator.list_subagents(None).await;
    assert_eq!(all_workers.len(), 2);
}

/// Test: get_subagent retrieves specific tracker
#[tokio::test]
async fn get_subagent_retrieves_tracker() {
    let coordinator = SubagentCoordinator::default();
    
    let metadata = SubagentMetadata {
        subagent_id: Uuid::new_v4(),
        parent_session_id: Some("session-1".to_string()),
        parent_prompt_id: None,
        subagent_type: "planner".to_string(),
        description: "plan the work".to_string(),
        run_in_background: false,
    };
    
    coordinator.spawn(metadata.clone()).await;
    
    let tracker = coordinator.get_subagent(metadata.subagent_id).await;
    
    assert!(tracker.is_some());
    let tracker = tracker.unwrap();
    assert_eq!(tracker.subagent_id, metadata.subagent_id);
    assert_eq!(tracker.subagent_type, "planner");
}

/// Test: get_subagent returns None for unknown ID
#[tokio::test]
async fn get_subagent_unknown_id() {
    let coordinator = SubagentCoordinator::default();
    
    let tracker = coordinator.get_subagent(Uuid::new_v4()).await;
    
    assert!(tracker.is_none());
}

/// Test: lifecycle states are correct
#[test]
fn subagent_states_are_terminal() {
    // Completed is terminal
    assert!(SubagentState::Completed { 
        output: "test".to_string(), 
        tool_calls: 1, 
        turns: 1 
    }.is_terminal());
    
    // Cancelled is terminal
    assert!(SubagentState::Cancelled { reason: None }.is_terminal());
    
    // Failed is terminal
    assert!(SubagentState::Failed { error: "error".to_string() }.is_terminal());
    
    // Orphaned is terminal
    assert!(SubagentState::Orphaned.is_terminal());
    
    // Running is not terminal
    assert!(!SubagentState::Running { 
        turn_count: 1, 
        tool_call_count: 1, 
        tokens_used: 100 
    }.is_terminal());
    
    // New is not terminal
    assert!(!SubagentState::New.is_terminal());
}

/// Test: tracker elapsed time
#[test]
fn tracker_elapsed_time() {
    let metadata = SubagentMetadata {
        subagent_id: Uuid::new_v4(),
        parent_session_id: None,
        parent_prompt_id: None,
        subagent_type: "test".to_string(),
        description: "test".to_string(),
        run_in_background: false,
    };
    
    let tracker = runie_agent::SubagentTracker::new(&metadata);
    
    // Should have some elapsed time (may be 0 or very small)
    let elapsed = tracker.elapsed();
    assert!(elapsed >= Duration::from_secs(0));
}
