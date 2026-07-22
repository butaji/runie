//! Tests for orphan subagent reconciliation (Task 26).
//!
//! These tests verify the OrphanedWorkerTracker correctly identifies and handles
//! orphaned swarm workers on session resume.

use runie_patterns::swarm::{
    OrphanedWorkerTracker, SwarmWorkerStatus,
};
use std::time::Duration;

/// Test 1: Normal subagent completes (not orphaned)
#[test]
fn normal_completion_not_orphaned() {
    let tracker = OrphanedWorkerTracker::new();
    
    // Spawn a worker
    tracker.spawn("worker-1".to_string(), "task 1".to_string());
    
    // Complete it normally
    tracker.complete("worker-1");
    
    let workers = tracker.workers();
    assert_eq!(workers.len(), 1);
    assert_eq!(workers[0].status, SwarmWorkerStatus::Completed);
    assert!(!tracker.has_orphans());
}

/// Test 2: Subagent stuck >5min on resume (marked cancelled/orphaned)
#[tokio::test]
async fn stuck_subagent_marked_orphaned_on_resume() {
    // Use short timeout for testing
    let tracker = OrphanedWorkerTracker::new();
    let max_age = Duration::from_millis(10);
    
    // Spawn a worker
    tracker.spawn("stuck-worker".to_string(), "stuck task".to_string());
    
    // Wait for the timeout to expire
    tokio::time::sleep(Duration::from_millis(20)).await;
    
    // Reconcile with empty live worker list (simulating session resume with dead workers)
    let orphaned_count = tracker.reconcile_orphans_by_max_age(max_age);
    
    assert_eq!(orphaned_count, 1, "Should mark one worker as orphaned");
    
    let workers = tracker.workers();
    assert_eq!(workers[0].status, SwarmWorkerStatus::Orphaned);
    assert!(tracker.has_orphans());
}

/// Test 3: Manual cleanup command works
#[test]
fn manual_cleanup_removes_orphaned_workers() {
    let tracker = OrphanedWorkerTracker::new();
    
    // Spawn multiple workers in different states
    tracker.spawn("running-1".to_string(), "task 1".to_string());
    tracker.spawn("orphaned-1".to_string(), "task 2".to_string());
    tracker.spawn("cancelled-1".to_string(), "task 3".to_string());
    
    // Mark some as orphaned and cancelled
    let _workers = tracker.workers();
    // Manually set status for testing
    tracker.complete("running-1");
    tracker.cancel("cancelled-1");
    
    // Reconcile orphans (the orphaned-1 worker)
    let orphaned = tracker.reconcile_orphans_by_max_age(Duration::from_secs(0));
    assert_eq!(orphaned, 1);
    
    // Verify initial state
    let counts_before = tracker.status_counts();
    assert_eq!(counts_before.orphaned, 1);
    assert_eq!(counts_before.cancelled, 1);
    assert_eq!(counts_before.completed, 1);
    assert_eq!(counts_before.running, 0);
    
    // Run cleanup
    let cleaned = tracker.cleanup_orphaned_workers();
    
    assert_eq!(cleaned, 2, "Should clean up orphaned and cancelled workers");
    
    let workers_after = tracker.workers();
    assert_eq!(workers_after.len(), 1, "Only completed worker should remain");
    assert_eq!(workers_after[0].id, "running-1");
}

/// Test: heartbeat tracking for orphan detection
#[test]
fn heartbeat_timeout_tracks_worker_staleness() {
    let tracker = OrphanedWorkerTracker::new();
    
    // Spawn worker with custom timeout
    tracker.spawn_with_timeout(
        "heartbeat-worker".to_string(), 
        "heartbeat task".to_string(),
        Duration::from_millis(50),
    );
    
    let workers = tracker.workers();
    assert_eq!(workers[0].heartbeat_timeout, Duration::from_millis(50));
    
    // After 10ms, worker should not be stale
    std::thread::sleep(Duration::from_millis(10));
    assert!(!workers[0].is_stale());
}

/// Test: live worker reconciliation (missing from live list)
#[test]
fn reconcile_missing_workers_are_orphaned() {
    let tracker = OrphanedWorkerTracker::new();
    
    // Spawn multiple workers
    tracker.spawn("alive-1".to_string(), "task 1".to_string());
    tracker.spawn("dead-1".to_string(), "task 2".to_string());
    tracker.spawn("dead-2".to_string(), "task 3".to_string());
    
    // Only "alive-1" is in the live list
    let live_ids = vec!["alive-1".to_string()];
    let orphaned = tracker.reconcile_orphans(&live_ids);
    
    assert_eq!(orphaned, 2, "Should mark dead workers as orphaned");
    
    let counts = tracker.status_counts();
    assert_eq!(counts.running, 1);
    assert_eq!(counts.orphaned, 2);
}

/// Test: full reconciliation combines both conditions
#[tokio::test]
async fn full_reconciliation_checks_both_conditions() {
    let tracker = OrphanedWorkerTracker::new();
    let max_age = Duration::from_millis(10);
    
    // Spawn workers: one live, one missing, one stale
    tracker.spawn("live".to_string(), "task 1".to_string());
    tracker.spawn("missing".to_string(), "task 2".to_string());
    tracker.spawn_with_timeout(
        "stale".to_string(), 
        "task 3".to_string(),
        Duration::from_millis(5),
    );
    
    // Wait for stale timeout
    tokio::time::sleep(Duration::from_millis(20)).await;
    
    // Only "live" is in the live list
    let live_ids = vec!["live".to_string()];
    let (orphaned, running) = tracker.reconcile_orphans_full(&live_ids, max_age);
    
    assert_eq!(orphaned, 2, "missing and stale should be orphaned");
    assert_eq!(running, 1, "only live should remain running");
}

/// Test: status counts
#[test]
fn status_counts_correct() {
    let tracker = OrphanedWorkerTracker::new();
    
    tracker.spawn("r1".to_string(), "task".to_string());
    tracker.spawn("r2".to_string(), "task".to_string());
    tracker.spawn("c1".to_string(), "task".to_string());
    tracker.spawn("f1".to_string(), "task".to_string());
    tracker.spawn("o1".to_string(), "task".to_string());
    
    tracker.complete("c1");
    tracker.fail("f1");
    tracker.cancel("o1");
    
    let counts = tracker.status_counts();
    assert_eq!(counts.running, 2);
    assert_eq!(counts.completed, 1);
    assert_eq!(counts.failed, 1);
    assert_eq!(counts.cancelled, 1);
    assert_eq!(counts.orphaned, 1);
}
