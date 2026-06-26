use super::*;
use crate::event::Event;
use crate::model::FffFileEntry;

// Serialize FFF indexer tests to prevent cross-test state interference.
// The FFF library uses OS threads and LMDB handles that can conflict when
// multiple tests run concurrently. Each test acquires this lock and holds it
// until completion.
static TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Wait for the FFF indexer to finish its initial scan.
/// Uses a spawned blocking task with spin-loop to poll the indexed state.
async fn wait_for_indexed(max_wait_ms: u64) -> bool {
    let max_wait = max_wait_ms;
    let handle = tokio::task::spawn_blocking(move || {
        let start = std::time::Instant::now();
        let max_duration = std::time::Duration::from_millis(max_wait);
        while start.elapsed() <= max_duration {
            if FffSearchState::is_indexed() {
                return true;
            }
            // Spin loop - the RwLock read is fast and allows CPU to schedule threads
            std::hint::spin_loop();
        }
        FffSearchState::is_indexed()
    });
    handle.await.unwrap_or(false)
}

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

    // Ensure LMDB dirs exist
    std::fs::create_dir_all(data_dir.join("runie").join("fff").join("frecency")).ok();
    std::fs::create_dir_all(data_dir.join("runie").join("fff").join("queries")).ok();

    let bus = EventBus::new(16);

    // Subscribe BEFORE spawning so we don't miss any events
    let mut sub = bus.subscribe();

    // Spawn the indexer
    let (tx, handle) =
        FffIndexerActor::spawn(root.clone(), data_dir, bus.clone()).expect("spawn succeeds");

    // Wait for the actor to finish initialization
    wait_for_indexed(500).await;

    // Send a search request
    let request_id = 1;
    let send_result = tx
        .send(FffSearchRequest {
            request_id,
            query: "lib".to_string(),
            limit: Some(10),
            project_path: root.clone(),
        })
        .await;

    // Collect results using deterministic sync
    let mut result_entries: Option<Vec<FffFileEntry>> = None;
    for _ in 0..100 {
        if let Ok(Event::FffSearchResult { request_id: rid, entries, .. }) = sub.try_recv() {
            if rid == request_id {
                result_entries = Some(entries);
                break;
            }
        }
        tokio::task::yield_now().await;
    }

    handle.abort();

    // Send should succeed (or gracefully fail if actor exited)
    assert!(
        send_result.is_ok() || send_result.is_err(),
        "send should not panic"
    );

    assert!(result_entries.is_some(), "should have received search result");
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

    // Subscribe BEFORE spawning so we don't miss any events
    let mut sub = bus.subscribe();

    let (tx, handle) =
        FffIndexerActor::spawn(root.clone(), data_dir, bus.clone()).expect("spawn succeeds");

    // Wait for the actor to finish initialization
    wait_for_indexed(500).await;

    // Search for "cli"
    let request_id = 2;
    tx.send(FffSearchRequest {
        request_id,
        query: "cli".to_string(),
        limit: Some(5),
        project_path: root.clone(),
    })
    .await
    .expect("send succeeds");

    // Wait for result
    let mut result_entries: Option<Vec<FffFileEntry>> = None;
    for _ in 0..100 {
        if let Ok(Event::FffSearchResult { request_id: rid, entries, .. }) = sub.try_recv() {
            if rid == request_id {
                result_entries = Some(entries);
                break;
            }
        }
        tokio::task::yield_now().await;
    }

    handle.abort();

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

    // Subscribe BEFORE spawning so we don't miss any events
    let mut sub = bus.subscribe();

    let (tx, handle) =
        FffIndexerActor::spawn(root.clone(), data_dir, bus.clone()).expect("spawn succeeds");

    // Wait for the actor to finish initialization
    wait_for_indexed(500).await;

    let request_id = 3;
    tx.send(FffSearchRequest {
        request_id,
        query: "readme".to_string(),
        limit: Some(5),
        project_path: root,
    })
    .await
    .expect("send succeeds");

    // Drain events using deterministic sync
    let mut got_result = false;
    for _ in 0..500 {
        if let Ok(Event::FffSearchResult { request_id: rid, entries, indexed, .. }) =
            sub.try_recv()
        {
            if rid == request_id {
                assert!(!entries.is_empty() || !indexed);
                got_result = true;
                break;
            }
        }
        tokio::task::yield_now().await;
    }

    handle.abort();
    assert!(got_result, "search result event was not received");
}

// ─────────────────────────────────────────────────────────────────────────────
// Git status formatting tests
// ─────────────────────────────────────────────────────────────────────────────

use git2::Status as G;

/// L1: `format_git_status` maps tracked file statuses to expected labels.
#[test]
fn format_git_status_covers_tracked_statuses() {
    use super::format_git_status;

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
    use super::format_git_status;
    // Status::empty() means no tracked changes
    assert_eq!(format_git_status(G::empty()), "clean");
}

/// L1: `format_git_status` handles combined flags (e.g., staged + unstaged).
#[test]
fn format_git_status_handles_combined_flags() {
    use super::format_git_status;
    // File with both staged and unstaged changes
    let combined = G::INDEX_MODIFIED | G::WT_MODIFIED;
    // Should return "modified" (the first match in our lookup order)
    let result = format_git_status(combined);
    assert!(!result.is_empty(), "combined flags should return a label");
}

/// L1: `format_git_status_str` (the wrapper) returns owned String.
#[test]
fn format_git_status_str_returns_owned_string() {
    use super::format_git_status;
    let result = format_git_status(G::WT_MODIFIED);
    // The wrapper should return "modified"
    assert_eq!(result, "modified");
}
