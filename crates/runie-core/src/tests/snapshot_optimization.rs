//! Snapshot optimization tests (Layer 1–3)

use crate::message::Part;
use crate::model::{AppState, ChatMessage, Role};
use crate::snapshot::Snapshot;
use crate::Event;
use std::sync::Arc;

// Layer 1 — State/Logic

#[test]
fn test_snapshot_contains_expected_fields() {
    let mut state = AppState::default();
    state.session.messages.push(ChatMessage {
        role: Role::User,
        timestamp: 1.0,
        id: "u1".into(),
        parts: vec![Part::Text {
            content: "hello".into(),
        }],
        ..Default::default()
    });
    state.refresh_after_message_change();

    let snap = state.snapshot();
    assert!(!snap.elements.is_empty(), "elements should be present");
    assert!(
        !snap.line_counts.is_empty(),
        "line_counts should be present"
    );
    assert_eq!(snap.input, state.input.input);
    assert_eq!(snap.cursor_pos, state.input.cursor_pos);
    assert_eq!(snap.provider, state.config.current_provider);
    assert_eq!(snap.model, state.config.current_model);
}

#[test]
fn test_cached_feed_reuse_on_gen_match() {
    // Verify cached_feed is reused when message_gen hasn't changed,
    // avoiding a redundant build_view_cache() call.
    let mut state = AppState::default();
    state.session.messages.push(ChatMessage {
        role: Role::User,
        timestamp: 1.0,
        id: "u1".into(),
        parts: vec![Part::Text {
            content: "hello".into(),
        }],
        ..Default::default()
    });
    state.refresh_after_message_change();

    // First snapshot: ensure_fresh() builds and stores the cache.
    let snap1 = state.snapshot();
    let gen1 = state.view().message_gen;
    assert!(!snap1.elements.is_empty());

    // Second snapshot with no state change: cache should be reused.
    let snap2 = state.snapshot();
    let gen2 = state.view().message_gen;
    assert_eq!(gen1, gen2);
    assert_eq!(snap1.elements.len(), snap2.elements.len());
    assert_eq!(snap1.total_lines, snap2.total_lines);

    // Third snapshot after a message change: cache should be rebuilt.
    state.session.messages.push(ChatMessage {
        role: Role::User,
        timestamp: 2.0,
        id: "u2".into(),
        parts: vec![Part::Text {
            content: "world".into(),
        }],
        ..Default::default()
    });
    state.refresh_after_message_change();
    let snap3 = state.snapshot();
    assert!(snap3.elements.len() > snap1.elements.len());
    assert!(state.view().message_gen > gen1);
}

#[test]
fn test_cached_feed_none_initially() {
    // Fresh AppState has no cached feed until ensure_fresh() is called.
    let state = AppState::default();
    assert!(state.view().cached_feed.is_none());
}

fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn test_snapshot_is_send_sync() {
    assert_send_sync::<Snapshot>();
}

// Layer 2 — Event Handling

#[test]
fn test_event_triggers_snapshot_update() {
    let mut state = AppState::default();
    state.update(crate::Event::Input('h'));
    state.update(crate::Event::Input('i'));
    state.update(Event::submit());
    state.ensure_fresh();

    let snap1 = state.snapshot();
    let count1 = snap1.elements.len();

    state.update(crate::Event::Response {
        id: "req.0".into(),
        content: "Hello back".into(),

        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),
    });
    state.ensure_fresh();

    let snap2 = state.snapshot();
    let count2 = snap2.elements.len();

    assert!(
        count2 > count1,
        "Snapshot should reflect new elements after agent response"
    );
}

// Layer 3 — Rendering

#[test]
fn test_render_receives_valid_snapshot() {
    let mut state = AppState::default();
    for i in 0..10 {
        state.session.messages.push(ChatMessage {
            role: Role::User,
            parts: vec![Part::Text {
                content: format!("msg{}", i),
            }],
            timestamp: i as f64,
            id: format!("u{}", i),
            ..Default::default()
        });
    }
    state.refresh_after_message_change();

    let snap = state.snapshot();
    let region = snap.visible(0, 5);
    assert!(
        region.len() <= snap.elements.len(),
        "Visible region should not exceed total elements"
    );
}

// Arc-specific optimization tests
// Note: Arc pointer stability is now handled by UiActor's cache.
// These tests verify correct output rather than internal optimization.

#[test]
fn test_elements_built_correctly() {
    let mut state = AppState::default();
    state.session.messages.push(ChatMessage {
        role: Role::User,
        timestamp: 1.0,
        id: "u1".into(),
        parts: vec![Part::Text {
            content: "hello".into(),
        }],
        ..Default::default()
    });
    state.refresh_after_message_change();

    let snap1 = state.snapshot();
    let snap2 = state.snapshot();

    // Elements should have correct content in both snapshots
    assert!(!snap1.elements.is_empty(), "Elements should be present");
    assert!(
        !snap2.elements.is_empty(),
        "Elements should be present in second snapshot"
    );
    // Element counts should match
    assert_eq!(snap1.elements.len(), snap2.elements.len());
}

#[test]
fn test_elements_correct_after_state_mutation() {
    let mut state = AppState::default();
    state.session.messages.push(ChatMessage {
        role: Role::User,
        timestamp: 1.0,
        id: "u1".into(),
        parts: vec![Part::Text {
            content: "first".into(),
        }],
        ..Default::default()
    });
    state.refresh_after_message_change();

    let snap1 = state.snapshot();
    let count1 = snap1.elements.len();

    // Mutate state but NOT messages (e.g., cursor move)
    state.update(crate::Event::Input('x'));
    state.ensure_fresh();

    let snap2 = state.snapshot();
    let count2 = snap2.elements.len();

    // Element count should be the same because messages didn't change
    assert_eq!(
        count1, count2,
        "Element count should be stable when messages are unchanged"
    );
}

#[test]
fn test_auth_providers_cached() {
    let mut state = AppState::default();
    state.ensure_fresh();

    let snap1 = state.snapshot();
    let snap2 = state.snapshot();
    assert!(
        Arc::ptr_eq(&snap1.auth_providers, &snap2.auth_providers),
        "auth_providers should be Arc-shared across unchanged snapshots"
    );
}

#[test]
fn test_settings_items_cached() {
    let mut state = AppState::default();
    state.ensure_fresh();

    let snap1 = state.snapshot();
    let snap2 = state.snapshot();
    assert!(
        Arc::ptr_eq(&snap1.settings_items, &snap2.settings_items),
        "settings_items should be Arc-shared across unchanged snapshots"
    );
}

#[test]
fn test_session_tree_items_cached() {
    let mut state = AppState::default();
    state.ensure_fresh();

    let snap1 = state.snapshot();
    let snap2 = state.snapshot();
    assert!(
        Arc::ptr_eq(&snap1.session_tree_items, &snap2.session_tree_items),
        "session_tree_items should be Arc-shared across unchanged snapshots"
    );
}

#[test]
fn test_palette_items_cached() {
    let mut state = AppState::default();
    state.ensure_fresh();

    let snap1 = state.snapshot();
    let snap2 = state.snapshot();
    assert!(
        Arc::ptr_eq(&snap1.palette_items, &snap2.palette_items),
        "palette_items should be Arc-shared across unchanged snapshots"
    );
}

// Layer 1 — State/Logic

#[test]
fn test_refresh_after_message_change_updates_flags() {
    let mut state = AppState::default();
    state.session.messages.push(ChatMessage {
        role: Role::User,
        timestamp: 1.0,
        id: "u1".into(),
        parts: vec![Part::Text {
            content: "hello".into(),
        }],
        ..Default::default()
    });

    let gen_before = state.view().message_gen;
    state.refresh_after_message_change();

    // Verify messages_changed behavior: message_gen should be bumped
    assert!(state.view().message_gen > gen_before);

    // Verify ensure_fresh was called: cache should be rebuilt, dirty should be cleared
    assert!(!state.view().dirty);
    // View cache is now built into Snapshot, not stored in ViewState
    let snap = state.snapshot();
    assert!(!snap.elements.is_empty());
}
