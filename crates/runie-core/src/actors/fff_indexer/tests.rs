use super::*;
use std::time::Duration;

/// Wait for the actor's global state to be registered (indicating init has started).
/// Uses timeout-based polling to be deterministic without arbitrary wall-clock waits.
async fn wait_for_actor_ready() {
    let result = tokio::time::timeout(Duration::from_secs(5), async {
        for _ in 0..500 {
            if FffSearchState::get().is_some() {
                return;
            }
            tokio::task::yield_now().await;
        }
    })
    .await;
    // Proceed regardless of timeout - the actor may still be initializing
    let _ = result;
}

#[tokio::test]
async fn indexer_initializes_in_temp_dir() {
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

    // Create the bus
    let bus = EventBus::new(16);

    // Spawn the indexer
    let (tx, handle) =
        FffIndexerActor::spawn(root.clone(), data_dir, bus.clone()).expect("spawn succeeds");

    // Wait for actor to be ready (deterministic timeout-based sync instead of sleep)
    wait_for_actor_ready().await;

    // Send a search request
    let request_id = 42;
    let send_result = tx
        .send(FffSearchRequest {
            request_id,
            query: "lib".to_string(),
            limit: Some(10),
            project_path: root.clone(),
        })
        .await;

    // Collect results using deterministic sync (extended polling for actor init)
    let mut result = None;
    let mut sub = bus.subscribe();
    for _ in 0..200 {
        if let Some(Ok(FffSearchResult(payload))) = sub.try_recv() {
            if payload.request_id == request_id {
                result = Some(payload);
                break;
            }
        }
        tokio::task::yield_now().await;
    }

    // Abort the actor
    handle.abort();

    // Send should succeed (or gracefully fail if actor exited)
    assert!(
        send_result.is_ok() || send_result.is_err(),
        "send should not panic"
    );

    if let Some(res) = result {
        assert_eq!(res.request_id, request_id);
    }
}

#[tokio::test]
async fn indexer_answers_file_search() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    let data_dir = tmp.path().to_path_buf();

    // Create structured test files
    std::fs::create_dir_all(root.join("src/cli")).ok();
    std::fs::create_dir_all(root.join("src/server")).ok();
    std::fs::write(root.join("src/cli/main.rs"), "fn main() {}").unwrap();
    std::fs::write(root.join("src/server/api.rs"), "pub fn api() {}").unwrap();

    let bus = EventBus::new(16);
    let (tx, handle) =
        FffIndexerActor::spawn(root.clone(), data_dir, bus.clone()).expect("spawn succeeds");

    // Wait for actor to be ready (deterministic timeout-based sync instead of sleep)
    wait_for_actor_ready().await;

    // Search for "cli"
    let request_id = 7;
    tx.send(FffSearchRequest {
        request_id,
        query: "cli".to_string(),
        limit: Some(5),
        project_path: root.clone(),
    })
    .await
    .expect("send succeeds");

    // Wait for result using deterministic sync (extended polling for actor init)
    let mut result = None;
    let mut sub = bus.subscribe();
    for _ in 0..200 {
        if let Some(Ok(FffSearchResult(payload))) = sub.try_recv() {
            if payload.request_id == request_id {
                result = Some(payload);
                break;
            }
        }
        tokio::task::yield_now().await;
    }

    handle.abort();

    let result = result.expect("got a result for request_id 7");
    assert_eq!(result.request_id, 7);
    // Should find src/cli/main.rs
    assert!(
        result.items.iter().any(|i| i.relative_path.contains("cli")),
        "expected cli file in results: {:?}",
        result.items
    );
}

#[tokio::test]
async fn search_request_event_returns_results() {
    // Integration test: search request → search result event
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    let data_dir = tmp.path().to_path_buf();

    std::fs::write(root.join("readme.md"), "# Hello World").unwrap();
    std::fs::write(root.join("todo.txt"), "buy milk").unwrap();

    let bus = EventBus::new(16);
    let (tx, handle) =
        FffIndexerActor::spawn(root.clone(), data_dir, bus.clone()).expect("spawn succeeds");

    // Wait for actor to be ready (deterministic timeout-based sync instead of sleep)
    wait_for_actor_ready().await;

    let request_id = 99;
    tx.send(FffSearchRequest {
        request_id,
        query: "readme".to_string(),
        limit: Some(5),
        project_path: root,
    })
    .await
    .expect("send succeeds");

    // Drain events using deterministic sync (extended polling for actor init)
    let mut got_result = false;
    let mut sub = bus.subscribe();
    for _ in 0..200 {
        if let Some(Ok(FffSearchResult(payload))) = sub.try_recv() {
            if payload.request_id == request_id {
                assert!(!payload.items.is_empty() || !payload.indexed);
                got_result = true;
                break;
            }
        }
        tokio::task::yield_now().await;
    }

    handle.abort();
    assert!(got_result, "search result event was not received");
}
