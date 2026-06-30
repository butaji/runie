//! Tests for `RactorSessionActor`.

use std::path::PathBuf;

use crate::actors::session::RactorSessionActor;
use crate::bus::{EventBus, Receiver};
use crate::trust::{TrustDecision, TrustManager};
use crate::Event;

fn make_test_store() -> tempfile::TempDir {
    tempfile::TempDir::new().unwrap()
}

/// Wait for an event matching a predicate with a deterministic timeout.
async fn wait_for_event<F>(sub: &mut Receiver<Event>, pred: F) -> bool
where
    F: Fn(&Event) -> bool,
{
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
    while tokio::time::Instant::now() < deadline {
        let timeout_duration = deadline - tokio::time::Instant::now();
        match tokio::time::timeout(timeout_duration, sub.recv()).await {
            Ok(Ok(evt)) => {
                if pred(&evt) {
                    return true;
                }
            }
            Ok(Err(_)) | Err(_) => break,
        }
    }
    false
}

#[tokio::test]
async fn session_actor_spawns_and_emits_events() {
    let _tmp = make_test_store();
    let bus = EventBus::<Event>::new(4);
    let mut sub = bus.subscribe();
    let (handle, _cell, _join) = RactorSessionActor::spawn(bus.clone()).await.unwrap();

    // Should emit initial events (trust and history)
    let saw_trust = wait_for_event(&mut sub, |e| matches!(e, Event::TrustLoaded { .. })).await;
    let saw_history = wait_for_event(&mut sub, |e| matches!(e, Event::HistoryLoaded { .. })).await;
    assert!(saw_trust, "should emit TrustLoaded");
    assert!(saw_history, "should emit HistoryLoaded");

    // Test trust setting via handle
    handle
        .set_trust(PathBuf::from("/tmp/project"), TrustDecision::Trusted)
        .await;

    let saw_changed = wait_for_event(&mut sub, |e| matches!(e, Event::TrustChanged { .. })).await;
    assert!(saw_changed, "should emit TrustChanged after set_trust");
}

#[tokio::test]
async fn session_actor_add_user_message_via_handle() {
    let bus = EventBus::<Event>::new(4);
    let (handle, _cell, _join) = RactorSessionActor::spawn(bus.clone()).await.unwrap();

    // Drain initial events
    let mut sub = bus.subscribe();
    let _ = tokio::time::timeout(std::time::Duration::from_millis(500), async {
        while sub.recv().await.is_ok() {}
    })
    .await;

    handle.try_add_user_message("hello".to_string(), vec![]);

    let saw_changed = wait_for_event(&mut sub, |e| matches!(e, Event::SessionChanged { .. })).await;
    assert!(
        saw_changed,
        "should emit SessionChanged after adding message"
    );
}

#[tokio::test]
async fn session_actor_reset_via_handle() {
    let bus = EventBus::<Event>::new(4);
    let (handle, _cell, _join) = RactorSessionActor::spawn(bus.clone()).await.unwrap();

    // Drain initial events
    let mut sub = bus.subscribe();
    let _ = tokio::time::timeout(std::time::Duration::from_millis(500), async {
        while sub.recv().await.is_ok() {}
    })
    .await;

    // Add a message first
    handle.try_add_user_message("hello".to_string(), vec![]);
    let _ = wait_for_event(&mut sub, |e| matches!(e, Event::SessionChanged { .. })).await;
    while sub.try_recv().is_ok() {}

    // Then reset
    handle.try_reset();

    let saw_changed = wait_for_event(&mut sub, |e| matches!(e, Event::SessionChanged { .. })).await;
    assert!(saw_changed, "should emit SessionChanged after reset");
}

#[test]
fn trust_manager_default_is_untrusted() {
    let manager = TrustManager::default();
    let path = PathBuf::from("/tmp/test");
    assert!(manager.decision_for(&path).is_none());
}
