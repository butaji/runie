//! Tests for the worktree pool.

use std::path::PathBuf;
use std::time::Duration;

use runie_core::worktree::WorktreePool;

#[tokio::test]
async fn pool_creation() {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let pool = WorktreePool::new(source.clone(), 2, 30);
    assert_eq!(pool.count_ready(), 0);
}

#[tokio::test]
async fn acquire_releases_handle() {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let pool = WorktreePool::new(source.clone(), 1, 30);
    let handle = pool.acquire_worktree().await;
    assert!(handle.is_ok());
    let h = handle.unwrap();
    assert!(h.path().exists());
}

#[tokio::test]
async fn handle_drop_releases_worktree() {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let pool = WorktreePool::new(source.clone(), 1, 30);
    let handle = pool.acquire_worktree().await.unwrap();
    let path = handle.path().to_path_buf();
    drop(handle);
    assert!(!path.exists() || path.join(".ready").exists());
}

#[tokio::test]
async fn count_ready_returns_count() {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let pool = WorktreePool::new(source.clone(), 2, 30);
    tokio::time::sleep(Duration::from_millis(500)).await;
    let count = pool.count_ready();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn cleanup_stale_returns_zero_when_fresh() {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let pool = WorktreePool::new(source.clone(), 1, 30);
    let removed = pool.cleanup_stale().await;
    assert_eq!(removed, 0);
}

#[tokio::test]
async fn release_worktree_drops_handle() {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let pool = WorktreePool::new(source.clone(), 1, 30);
    let handle = pool.acquire_worktree().await.unwrap();
    pool.release_worktree(handle);
}

#[tokio::test]
async fn multiple_acquires_same_pool() {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let pool = WorktreePool::new(source.clone(), 2, 30);
    let h1 = pool.acquire_worktree().await;
    let h2 = pool.acquire_worktree().await;
    assert!(h1.is_ok());
    assert!(h2.is_ok());
}
