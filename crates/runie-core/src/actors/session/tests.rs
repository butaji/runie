//! Tests for `RactorSessionActor`.

use std::path::PathBuf;

use tempfile::TempDir;

use crate::actors::session::RactorSessionActor;
use crate::actors::RactorSessionHandle;
use crate::bus::EventBus;
use crate::model::SessionState;
use crate::trust::{TrustDecision, TrustManager};
use crate::Event;

fn make_test_store() -> tempfile::TempDir {
    tempfile::TempDir::new().unwrap()
}

#[tokio::test]
async fn session_actor_spawns_and_emits_events() {
    let _tmp = make_test_store();
    let bus = EventBus::<Event>::new(4);
    let mut sub = bus.subscribe();
    let (handle, _cell) = RactorSessionActor::spawn(bus.clone()).await.unwrap();

    // Should emit initial events (trust and history)
    let mut saw_trust = false;
    let mut saw_history = false;
    for _ in 0..60 {
        if saw_trust && saw_history {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        while let Ok(evt) = sub.try_recv() {
            match evt {
                Event::TrustLoaded { .. } => saw_trust = true,
                Event::HistoryLoaded { .. } => saw_history = true,
                _ => {}
            }
        }
    }
    assert!(saw_trust, "should emit TrustLoaded");
    assert!(saw_history, "should emit HistoryLoaded");

    // Test trust setting via handle
    handle
        .set_trust(PathBuf::from("/tmp/project"), TrustDecision::Trusted)
        .await;

    let mut saw_changed = false;
    for _ in 0..60 {
        if saw_changed {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        while let Ok(evt) = sub.try_recv() {
            if matches!(evt, Event::TrustChanged { .. }) {
                saw_changed = true;
            }
        }
    }
    assert!(saw_changed, "should emit TrustChanged after set_trust");
}

#[tokio::test]
async fn session_actor_add_user_message_via_handle() {
    let bus = EventBus::<Event>::new(4);
    let (handle, _cell) = RactorSessionActor::spawn(bus.clone()).await.unwrap();

    // Drain initial events (with timeout to prevent infinite hang)
    let mut sub = bus.subscribe();
    let _ = tokio::time::timeout(std::time::Duration::from_millis(500), async {
        while sub.recv().await.is_ok() {}
    }).await;

    handle.try_add_user_message("hello".to_string(), vec![]);

    // Give actor time to process
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let mut saw_changed = false;
    while let Ok(evt) = sub.try_recv() {
        if matches!(evt, Event::SessionChanged { .. }) {
            saw_changed = true;
        }
    }
    assert!(saw_changed, "should emit SessionChanged after adding message");
}

#[tokio::test]
async fn session_actor_reset_via_handle() {
    let bus = EventBus::<Event>::new(4);
    let (handle, _cell) = RactorSessionActor::spawn(bus.clone()).await.unwrap();

    // Drain initial events (with timeout to prevent infinite hang)
    let mut sub = bus.subscribe();
    let _ = tokio::time::timeout(std::time::Duration::from_millis(500), async {
        while sub.recv().await.is_ok() {}
    }).await;

    // Add a message first
    handle.try_add_user_message("hello".to_string(), vec![]);
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    while let Ok(_) = sub.try_recv() {}

    // Then reset
    handle.try_reset();

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let mut saw_changed = false;
    while let Ok(evt) = sub.try_recv() {
        if matches!(evt, Event::SessionChanged { .. }) {
            saw_changed = true;
        }
    }
    assert!(saw_changed, "should emit SessionChanged after reset");
}

#[test]
fn trust_manager_default_is_untrusted() {
    let manager = TrustManager::default();
    // Default trust manager should start empty (untrusted for all)
    let path = PathBuf::from("/tmp/test");
    assert!(manager.decision_for(&path).is_none());
}
