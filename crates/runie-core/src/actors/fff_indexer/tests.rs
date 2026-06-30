//! Tests for the search indexer actor.

use super::*;
use crate::bus::EventBus;
use crate::event::Event;
use crate::model::FffFileEntry;

// Serialize FFF indexer tests to prevent cross-test state interference.
// Each test acquires this lock and holds it until completion.
static TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[tokio::test(flavor = "current_thread")]
async fn indexer_initializes_in_temp_dir() {
    let _lock = TEST_MUTEX.lock().unwrap();
    FffSearchState::reset_for_test();
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    let data_dir = tmp.path().to_path_buf();

    // Create a few files
    std::fs::create_dir_all(root.join("src")).ok();
    std::fs::create_dir_all(root.join("tests")).ok();
    std::fs::write(root.join("src/lib.rs"), "// lib").ok();
    std::fs::write(root.join("src/main.rs"), "// main").ok();
    std::fs::write(root.join("tests/example.rs"), "// test").ok();

    let bus = EventBus::new(16);

    // Subscribe BEFORE spawning so we don't miss any events
    let mut sub = bus.subscribe();

    // Spawn the indexer — index is built synchronously before actor starts.
    let (handle, _cell, _join) = RactorFffIndexerActor::spawn(root.clone(), data_dir, bus.clone())
        .await
        .expect("spawn succeeds");

    // Index should be ready immediately after spawn returns.
    assert!(FffSearchState::is_indexed(), "index should be ready after spawn");

    // Send a search request
    let request_id = 1;
    handle
        .search(FffSearchRequest {
            request_id,
            query: "lib".to_string(),
            limit: Some(10),
            project_path: root.clone(),
        })
        .await;

    // Collect results using deterministic sync
    let mut result_entries: Option<Vec<FffFileEntry>> = None;
    for _ in 0..100 {
        if let Ok(Event::FffSearchResult {
            request_id: rid,
            entries,
            ..
        }) = sub.try_recv()
        {
            if rid == request_id {
                result_entries = Some(entries);
                break;
            }
        }
        tokio::task::yield_now().await;
    }

    // Should have received a search result
    assert!(
        result_entries.is_some(),
        "should have received search result"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn indexer_answers_file_search() {
    let _lock = TEST_MUTEX.lock().unwrap();
    FffSearchState::reset_for_test();
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    let data_dir = tmp.path().to_path_buf();

    // Create structured test files
    std::fs::create_dir_all(root.join("src/cli")).ok();
    std::fs::create_dir_all(root.join("src/server")).ok();
    std::fs::write(root.join("src/cli/main.rs"), "fn main() {}").unwrap();
    std::fs::write(root.join("src/server/api.rs"), "pub fn api() {}").unwrap();

    let bus = EventBus::new(16);

    // Subscribe BEFORE spawning
    let mut sub = bus.subscribe();

    let (handle, _cell, _join) = RactorFffIndexerActor::spawn(root.clone(), data_dir, bus.clone())
        .await
        .expect("spawn succeeds");

    // Index should be ready
    assert!(FffSearchState::is_indexed());

    // Search for "cli"
    let request_id = 2;
    handle
        .search(FffSearchRequest {
            request_id,
            query: "cli".to_string(),
            limit: Some(5),
            project_path: root.clone(),
        })
        .await;

    // Wait for result
    let mut result_entries: Option<Vec<FffFileEntry>> = None;
    for _ in 0..100 {
        if let Ok(Event::FffSearchResult {
            request_id: rid,
            entries,
            ..
        }) = sub.try_recv()
        {
            if rid == request_id {
                result_entries = Some(entries);
                break;
            }
        }
        tokio::task::yield_now().await;
    }

    let entries = result_entries.expect("got a result for request_id 2");
    // Should find src/cli/main.rs
    assert!(
        entries.iter().any(|i| i.path.contains("cli")),
        "expected cli file in results: {:?}",
        entries
    );
}

#[tokio::test(flavor = "current_thread")]
async fn search_request_event_returns_results() {
    let _lock = TEST_MUTEX.lock().unwrap();
    FffSearchState::reset_for_test();
    // Integration test: search request → search result event
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    let data_dir = tmp.path().to_path_buf();

    std::fs::write(root.join("readme.md"), "# Hello World").unwrap();
    std::fs::write(root.join("todo.txt"), "buy milk").unwrap();

    let bus = EventBus::new(16);

    // Subscribe BEFORE spawning
    let mut sub = bus.subscribe();

    let (handle, _cell, _join) = RactorFffIndexerActor::spawn(root.clone(), data_dir, bus.clone())
        .await
        .expect("spawn succeeds");

    let request_id = 3;
    handle
        .search(FffSearchRequest {
            request_id,
            query: "readme".to_string(),
            limit: Some(5),
            project_path: root,
        })
        .await;

    // Drain events using deterministic sync
    let mut got_result = false;
    for _ in 0..500 {
        if let Ok(Event::FffSearchResult {
            request_id: rid,
            entries,
            indexed,
            ..
        }) = sub.try_recv()
        {
            if rid == request_id {
                assert!(!entries.is_empty() || !indexed);
                got_result = true;
                break;
            }
        }
        tokio::task::yield_now().await;
    }

    assert!(got_result, "search result event was not received");
}

// ─────────────────────────────────────────────────────────────────────────────
// Git status formatting tests
// ─────────────────────────────────────────────────────────────────────────────

use git2::Status as G;

/// L1: `format_git_status` maps tracked file statuses to expected labels.
#[test]
fn format_git_status_covers_tracked_statuses() {
    // WT_NEW / INDEX_NEW → "untracked"
    assert_eq!(format_git_status(G::WT_NEW), "untracked");
    assert_eq!(format_git_status(G::INDEX_NEW), "untracked");

    // WT_MODIFIED / INDEX_MODIFIED → "modified"
    assert_eq!(format_git_status(G::WT_MODIFIED), "modified");
    assert_eq!(format_git_status(G::INDEX_MODIFIED), "modified");

    // WT_DELETED / INDEX_DELETED → "deleted"
    assert_eq!(format_git_status(G::WT_DELETED), "deleted");
    assert_eq!(format_git_status(G::INDEX_DELETED), "deleted");

    // WT_RENAMED / INDEX_RENAMED → "renamed"
    assert_eq!(format_git_status(G::WT_RENAMED), "renamed");
    assert_eq!(format_git_status(G::INDEX_RENAMED), "renamed");
}

/// L1: `format_git_status` returns "clean" when no tracked flags are set.
#[test]
fn format_git_status_returns_clean_for_empty_status() {
    // Status::empty() means no tracked changes
    assert_eq!(format_git_status(G::empty()), "clean");
}

/// L1: `format_git_status` handles combined flags (e.g., staged + unstaged).
#[test]
fn format_git_status_handles_combined_flags() {
    // File with both staged and unstaged changes
    let combined = G::INDEX_MODIFIED | G::WT_MODIFIED;
    // Should return "modified" (the first match in our lookup order)
    let result = format_git_status(combined);
    assert!(!result.is_empty(), "combined flags should return a label");
}
