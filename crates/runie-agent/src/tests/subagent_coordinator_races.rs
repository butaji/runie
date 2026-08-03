//! Unit tests for subagent coordinator race safety (task 23).
//!
//! Regression tests for the TOCTOU counter underflow races between terminal
//! methods (`complete`, `fail`, `cancel`) and `detect_orphans`. The fix adds a
//! `has_been_counted: bool` guard inside each `SubagentEntry` so that only the
//! first terminal transition to observe `has_been_counted == false` decrements
//! `running_count` and sets the flag — subsequent transitions skip the decrement.

use crate::subagent_coordinator::{SubagentCoordinator, SubagentMetadata};
use uuid::Uuid;

fn make_metadata() -> SubagentMetadata {
    SubagentMetadata {
        subagent_id: Uuid::new_v4(),
        parent_session_id: None,
        parent_prompt_id: None,
        subagent_type: "test".to_string(),
        description: "race test".to_string(),
        run_in_background: false,
    }
}

/// Regression (task 23): concurrent `complete` + `detect_orphans` on the same
/// entry must not underflow `running_count`.
#[tokio::test]
async fn concurrent_complete_and_detect_orphans_no_underflow() {
    let coord = SubagentCoordinator::new(std::time::Duration::from_secs(0));
    let meta = make_metadata();
    let handle = coord.spawn(meta.clone()).await;
    assert_eq!(coord.running_count(), 1);

    // Call complete and detect_orphans concurrently on the same subagent.
    let (tx1, rx1) = tokio::sync::oneshot::channel();
    let (tx2, rx2) = tokio::sync::oneshot::channel();
    let id = handle.subagent_id;

    let coord_arc = std::sync::Arc::new(coord);
    let coord_clone = std::sync::Arc::clone(&coord_arc);
    let t1 = tokio::spawn(async move {
        let _ = tx1.send(());
        coord_clone.complete(id, "ok".to_string(), 0, 0).await;
    });

    let coord_clone2 = std::sync::Arc::clone(&coord_arc);
    let t2 = tokio::spawn(async move {
        // Wait for complete to start, then trigger orphan detection.
        let _ = rx1.await;
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        let _ = coord_clone2.detect_orphans().await;
        let _ = tx2.send(());
    });

    t2.await.unwrap();
    t1.await.unwrap();

    // Must not underflow: exactly 0 (never goes negative).
    assert_eq!(
        coord_arc.running_count(),
        0,
        "running_count must be exactly 0 after complete; no underflow"
    );
}

/// Regression (task 23): concurrent `fail` + `detect_orphans` on the same entry.
#[tokio::test]
async fn concurrent_fail_and_detect_orphans_no_underflow() {
    let coord = SubagentCoordinator::new(std::time::Duration::from_secs(0));
    let meta = make_metadata();
    let handle = coord.spawn(meta.clone()).await;
    assert_eq!(coord.running_count(), 1);

    let id = handle.subagent_id;
    let (tx1, rx1) = tokio::sync::oneshot::channel();

    let coord_arc = std::sync::Arc::new(coord);
    let coord_clone = std::sync::Arc::clone(&coord_arc);
    let t1 = tokio::spawn(async move {
        let _ = tx1.send(());
        coord_clone.fail(id, "boom".to_string()).await;
    });

    let coord_clone2 = std::sync::Arc::clone(&coord_arc);
    let t2 = tokio::spawn(async move {
        let _ = rx1.await;
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        let _ = coord_clone2.detect_orphans().await;
    });

    t2.await.unwrap();
    t1.await.unwrap();

    assert_eq!(
        coord_arc.running_count(),
        0,
        "no underflow after fail + detect_orphans"
    );
}

/// Regression (task 23): concurrent `cancel` + `detect_orphans` on the same entry.
#[tokio::test]
async fn concurrent_cancel_and_detect_orphans_no_underflow() {
    let coord = SubagentCoordinator::new(std::time::Duration::from_secs(0));
    let meta = make_metadata();
    let handle = coord.spawn(meta.clone()).await;
    assert_eq!(coord.running_count(), 1);

    let id = handle.subagent_id;
    let (tx1, rx1) = tokio::sync::oneshot::channel();

    let coord_arc = std::sync::Arc::new(coord);
    let coord_clone = std::sync::Arc::clone(&coord_arc);
    let t1 = tokio::spawn(async move {
        let _ = tx1.send(());
        coord_clone.cancel(id, None).await;
    });

    let coord_clone2 = std::sync::Arc::clone(&coord_arc);
    let t2 = tokio::spawn(async move {
        let _ = rx1.await;
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        let _ = coord_clone2.detect_orphans().await;
    });

    t2.await.unwrap();
    t1.await.unwrap();

    assert_eq!(
        coord_arc.running_count(),
        0,
        "no underflow after cancel + detect_orphans"
    );
}

/// Regression (task 23): multiple terminal calls racing on the same entry must
/// not decrement more than once.
#[tokio::test]
async fn multiple_concurrent_terminal_calls_same_entry() {
    let coord = SubagentCoordinator::new(std::time::Duration::from_secs(0));
    let meta = make_metadata();
    let handle = coord.spawn(meta.clone()).await;
    assert_eq!(coord.running_count(), 1);

    let id = handle.subagent_id;

    let coord_arc = std::sync::Arc::new(coord);
    let coord1 = std::sync::Arc::clone(&coord_arc);
    let coord2 = std::sync::Arc::clone(&coord_arc);
    let coord3 = std::sync::Arc::clone(&coord_arc);

    // Fire all three terminal paths simultaneously.
    let _ = tokio::join!(
        tokio::spawn(async move {
            coord1.complete(id, "ok".to_string(), 0, 0).await;
        }),
        tokio::spawn(async move {
            // Wait a tiny bit so complete() acquires the write lock first.
            tokio::time::sleep(std::time::Duration::from_micros(100)).await;
            coord2.fail(id, "err".to_string()).await;
        }),
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_micros(200)).await;
            coord3.cancel(id, None).await;
        }),
    );

    assert_eq!(
        coord_arc.running_count(),
        0,
        "final count must be 0, not underflowed"
    );
}

/// Sanity: normal flow (spawn N, complete M, fail K) gives exact count.
#[tokio::test]
async fn normal_flow_exact_count() {
    let coord = SubagentCoordinator::new(std::time::Duration::from_secs(300));

    let ids: Vec<_> = futures::future::join_all((0..5).map(|_| coord.spawn(make_metadata())))
        .await
        .into_iter()
        .map(|h| h.subagent_id)
        .collect();

    assert_eq!(coord.running_count(), 5);

    // Complete 2.
    coord.complete(ids[0], "ok".to_string(), 0, 0).await;
    coord.complete(ids[1], "ok".to_string(), 0, 0).await;
    assert_eq!(coord.running_count(), 3);

    // Fail 1.
    coord.fail(ids[2], "err".to_string()).await;
    assert_eq!(coord.running_count(), 2);

    // Cancel 1.
    coord.cancel(ids[3], None).await;
    assert_eq!(coord.running_count(), 1);

    // Leave 1 orphan.
    let _ = coord.detect_orphans().await;
    assert_eq!(
        coord.running_count(),
        1,
        "only 1 still running; orphan detection must not decrement"
    );
}
